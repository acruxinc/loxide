//! The core [`Logger`] type.

use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use crate::config::{Config, Format, Level, WriterKind};
use crate::json::JsonValue;
use crate::time::UtcDateTime;
use crate::{pretty, redact};

/// Field names owned by the record envelope; user fields using these names are
/// dropped so they can never shadow or duplicate the built-in fields.
const RESERVED_FIELDS: [&str; 4] = ["time", "level", "message", "caller"];

/// A cheap-to-clone, thread-safe structured logger.
///
/// A `Logger` bundles a [`Config`] with a set of persistent fields and a shared
/// output sink. Cloning (including via the `with_*` sub-logger methods) is cheap:
/// the output sink is shared through an [`Arc`], while each clone keeps its own
/// copy of the accumulated fields.
///
/// ```
/// use loxide::{Config, Level, Logger, json};
///
/// let logger = Logger::new(Config::default().with_level(Level::Debug));
/// logger.info("service started", &[("port", json!(8080))]);
///
/// // A sub-logger carries extra context on every subsequent record.
/// let request = logger.with_request_id("req-42");
/// request.debug("handling request", &[("path", json!("/health"))]);
/// ```
#[derive(Clone)]
pub struct Logger {
    config: Config,
    fields: Vec<(String, JsonValue)>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    writer_kind: WriterKind,
}

impl Logger {
    /// Creates a logger that writes to standard error (the conventional
    /// destination for diagnostics).
    pub fn new(config: Config) -> Self {
        Self::with_writer_kind(config, Box::new(io::stderr()), WriterKind::Stderr)
    }

    /// Creates a logger that writes to standard output.
    pub fn stdout(config: Config) -> Self {
        Self::with_writer_kind(config, Box::new(io::stdout()), WriterKind::Stdout)
    }

    /// Creates a logger writing to an arbitrary sink. Useful for writing to a
    /// file, an in-memory buffer, or a socket — and for capturing output in
    /// tests.
    pub fn with_writer(config: Config, writer: Box<dyn Write + Send>) -> Self {
        Self::with_writer_kind(config, writer, WriterKind::Other)
    }

    fn with_writer_kind(
        config: Config,
        writer: Box<dyn Write + Send>,
        writer_kind: WriterKind,
    ) -> Self {
        Logger {
            config,
            fields: Vec::new(),
            writer: Arc::new(Mutex::new(writer)),
            writer_kind,
        }
    }

    /// Returns the logger's configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns `true` if a record at `level` would be emitted (i.e. it meets the
    /// configured minimum level). Use this to guard expensive field computation.
    pub fn enabled(&self, level: Level) -> bool {
        level >= self.config.level
    }

    /// Logs `msg` at `level` with the given extra `fields`.
    pub fn log(&self, level: Level, msg: &str, fields: &[(&str, JsonValue)]) {
        if self.enabled(level) {
            self.emit(level, msg, fields, None);
        }
    }

    /// Logs `msg` at `level`, recording an explicit caller location. This is the
    /// entry point used by the `log_*!` macros; the location is only rendered
    /// when [`Config::caller`] is enabled.
    pub fn log_with_caller(
        &self,
        level: Level,
        msg: &str,
        fields: &[(&str, JsonValue)],
        file: &str,
        line: u32,
    ) {
        if self.enabled(level) {
            self.emit(level, msg, fields, Some((file, line)));
        }
    }

    /// Renders and writes a single record.
    fn emit(
        &self,
        level: Level,
        msg: &str,
        extra: &[(&str, JsonValue)],
        caller: Option<(&str, u32)>,
    ) {
        let now = UtcDateTime::now();
        let fields = self.merged_fields(extra);
        let caller = if self.config.caller { caller } else { None };

        let line = match self.config.resolved_format(self.writer_kind) {
            Format::Pretty => self.render_pretty(level, msg, &fields, caller, &now),
            // `Auto` is resolved before we get here; anything non-pretty is JSON.
            _ => Self::render_json(level, msg, &fields, caller, &now),
        };

        // A single locked `writeln!` keeps concurrent records from interleaving.
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writeln!(writer, "{line}");
        }
    }

    /// Merges persistent and per-call fields (per-call wins), strips reserved
    /// names, and applies redaction once so both formats stay consistent.
    fn merged_fields(&self, extra: &[(&str, JsonValue)]) -> Vec<(String, JsonValue)> {
        let mut merged = self.fields.clone();
        for (key, value) in extra {
            merged.retain(|(k, _)| k != *key);
            merged.push(((*key).to_string(), value.clone()));
        }
        merged.retain(|(k, _)| !RESERVED_FIELDS.contains(&k.as_str()));
        merged
            .into_iter()
            .map(|(key, value)| {
                let value = redact::redact_field(&key, value);
                (key, value)
            })
            .collect()
    }

    /// Renders a record as one compact JSON object.
    fn render_json(
        level: Level,
        msg: &str,
        fields: &[(String, JsonValue)],
        caller: Option<(&str, u32)>,
        now: &UtcDateTime,
    ) -> String {
        let mut object: Vec<(String, JsonValue)> = Vec::with_capacity(fields.len() + 4);
        object.push(("time".to_string(), JsonValue::from(now.to_iso8601())));
        object.push(("level".to_string(), JsonValue::from(level.as_str())));
        object.push(("message".to_string(), JsonValue::from(msg)));
        for (key, value) in fields {
            object.push((key.clone(), value.clone()));
        }
        if let Some((file, line)) = caller {
            object.push((
                "caller".to_string(),
                JsonValue::from(format!("{file}:{line}")),
            ));
        }
        JsonValue::Object(object).to_string()
    }

    /// Renders a record as a colored, human-readable line.
    fn render_pretty(
        &self,
        level: Level,
        msg: &str,
        fields: &[(String, JsonValue)],
        caller: Option<(&str, u32)>,
        now: &UtcDateTime,
    ) -> String {
        let nc = self.config.no_color;
        let mut parts = Vec::with_capacity(fields.len() + 4);

        parts.push(pretty::format_timestamp(&now.to_pretty(), nc));
        parts.push(pretty::format_level(level, nc));
        parts.push(msg.to_string());

        // Surface any `error` field prominently, before the rest.
        let mut remaining = fields.to_vec();
        if let Some(idx) = remaining.iter().position(|(k, _)| k == "error") {
            let (_, error) = remaining.remove(idx);
            parts.push(pretty::format_error_field(&scalar_text(&error), nc));
        }
        for (key, value) in &remaining {
            parts.push(pretty::format_field(key, &scalar_text(value), nc));
        }

        if let Some((file, line)) = caller {
            parts.push(pretty::format_caller(&pretty::short_caller(file, line), nc));
        }

        parts.join(" ")
    }

    // --- Sub-loggers ------------------------------------------------------

    /// Returns a sub-logger that tags every record with a `component` field.
    #[must_use]
    pub fn with_component(&self, name: &str) -> Logger {
        self.with_field("component", JsonValue::from(name))
    }

    /// Returns a sub-logger that tags every record with a `request_id` field.
    #[must_use]
    pub fn with_request_id(&self, id: &str) -> Logger {
        self.with_field("request_id", JsonValue::from(id))
    }

    /// Returns a new logger with the given trace ID attached to all records.
    #[must_use]
    pub fn with_trace_id(&self, id: &str) -> Logger {
        self.with_field("trace_id", JsonValue::from(id))
    }

    /// Generates a new UUIDv7 trace ID and attaches it to the logger.
    ///
    /// Requires the `uuid` feature.
    #[cfg(feature = "uuid")]
    #[must_use]
    pub fn with_new_trace_id(&self) -> Logger {
        self.with_field(
            "trace_id",
            JsonValue::from(uuid::Uuid::now_v7().to_string()),
        )
    }

    /// Returns a sub-logger carrying one additional persistent field. The new
    /// logger shares this logger's output sink.
    #[must_use]
    pub fn with_field(&self, key: &str, value: JsonValue) -> Logger {
        let mut fields = self.fields.clone();
        fields.retain(|(k, _)| k != key);
        fields.push((key.to_string(), value));
        Logger {
            config: self.config.clone(),
            fields,
            writer: Arc::clone(&self.writer),
            writer_kind: self.writer_kind,
        }
    }

    // --- Level shortcuts --------------------------------------------------

    /// Logs at [`Level::Trace`].
    pub fn trace(&self, msg: &str, fields: &[(&str, JsonValue)]) {
        self.log(Level::Trace, msg, fields);
    }

    /// Logs at [`Level::Debug`].
    pub fn debug(&self, msg: &str, fields: &[(&str, JsonValue)]) {
        self.log(Level::Debug, msg, fields);
    }

    /// Logs at [`Level::Info`].
    pub fn info(&self, msg: &str, fields: &[(&str, JsonValue)]) {
        self.log(Level::Info, msg, fields);
    }

    /// Logs at [`Level::Success`].
    pub fn success(&self, msg: &str, fields: &[(&str, JsonValue)]) {
        self.log(Level::Success, msg, fields);
    }

    /// Logs at [`Level::Warn`].
    pub fn warn(&self, msg: &str, fields: &[(&str, JsonValue)]) {
        self.log(Level::Warn, msg, fields);
    }

    /// Logs at [`Level::Error`].
    pub fn error(&self, msg: &str, fields: &[(&str, JsonValue)]) {
        self.log(Level::Error, msg, fields);
    }

    /// Logs at [`Level::Fatal`].
    pub fn fatal(&self, msg: &str, fields: &[(&str, JsonValue)]) {
        self.log(Level::Fatal, msg, fields);
    }
}

impl fmt::Debug for Logger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Logger")
            .field("config", &self.config)
            .field("fields", &self.fields)
            .finish_non_exhaustive()
    }
}

/// Renders a [`JsonValue`] for pretty output: strings are shown bare, everything
/// else falls back to its JSON representation.
fn scalar_text(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;
    use crate::json::from_json_str;

    /// A writer backed by a shared buffer, so tests can inspect the output.
    #[derive(Clone)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn logger(level: Level, format: Format) -> (Logger, Arc<Mutex<Vec<u8>>>) {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let config = Config::default()
            .with_level(level)
            .with_format(format)
            .with_no_color(true);
        (
            Logger::with_writer(config, Box::new(SharedBuf(buf.clone()))),
            buf,
        )
    }

    fn text(buf: &Arc<Mutex<Vec<u8>>>) -> String {
        String::from_utf8(buf.lock().unwrap().clone()).unwrap()
    }

    fn last_json(buf: &Arc<Mutex<Vec<u8>>>) -> JsonValue {
        from_json_str(text(buf).trim().lines().last().unwrap()).unwrap()
    }

    #[test]
    fn json_output_carries_fields() {
        let (l, buf) = logger(Level::Info, Format::Json);
        l.info("hello", &[("key", json!("value"))]);
        let data = last_json(&buf);
        assert_eq!(data["message"], "hello");
        assert_eq!(data["level"], "info");
        assert_eq!(data["key"], "value");
        assert!(data["time"].as_str().unwrap().ends_with('Z'));
    }

    #[test]
    fn level_filtering_drops_low_records() {
        let (l, buf) = logger(Level::Warn, Format::Json);
        l.info("filtered", &[]);
        assert!(text(&buf).is_empty());
        assert!(!l.enabled(Level::Info));
        l.warn("kept", &[]);
        assert!(!text(&buf).is_empty());
    }

    #[test]
    fn pretty_output_has_badge_and_fields() {
        let (l, buf) = logger(Level::Info, Format::Pretty);
        l.info("Server started", &[("port", json!(8080))]);
        let out = text(&buf);
        assert!(out.contains("INF"));
        assert!(out.contains("Server started"));
        assert!(out.contains("port=8080"));
    }

    #[test]
    fn pretty_output_has_no_ansi_when_disabled() {
        let (l, buf) = logger(Level::Info, Format::Pretty);
        l.info("test", &[]);
        assert!(!text(&buf).contains('\x1b'));
    }

    #[test]
    fn sub_loggers_accumulate_context() {
        let (l, buf) = logger(Level::Info, Format::Json);
        let sub = l.with_component("auth").with_request_id("req-1");
        sub.info("test", &[]);
        let data = last_json(&buf);
        assert_eq!(data["component"], "auth");
        assert_eq!(data["request_id"], "req-1");
    }

    #[test]
    fn sub_logger_context_persists_across_calls() {
        let (l, buf) = logger(Level::Info, Format::Json);
        let sub = l.with_component("db");
        sub.info("first", &[]);
        sub.warn("second", &[]);
        let out = text(&buf);
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(from_json_str(lines[0]).unwrap()["component"], "db");
        assert_eq!(from_json_str(lines[1]).unwrap()["component"], "db");
    }

    #[test]
    fn caller_is_recorded_when_enabled() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let config = Config::default()
            .with_format(Format::Json)
            .with_caller(true)
            .with_no_color(true);
        let l = Logger::with_writer(config, Box::new(SharedBuf(buf.clone())));
        l.log_with_caller(Level::Info, "test", &[], file!(), line!());
        assert!(
            last_json(&buf)["caller"]
                .as_str()
                .unwrap()
                .contains("logger.rs")
        );
    }

    #[test]
    fn per_call_fields_override_persistent_ones() {
        let (l, buf) = logger(Level::Info, Format::Json);
        l.with_field("stage", json!("a"))
            .info("m", &[("stage", json!("b"))]);
        assert_eq!(last_json(&buf)["stage"], "b");
    }

    #[test]
    fn sensitive_fields_are_redacted_in_json() {
        // This is the important guarantee: secrets never reach JSON output raw.
        let (l, buf) = logger(Level::Info, Format::Json);
        l.info("login", &[("password", json!("super_secret"))]);
        let data = last_json(&buf);
        assert_ne!(data["password"], "super_secret");
        assert_eq!(data["password"], "s**********t");
    }

    #[test]
    fn sensitive_fields_are_redacted_in_pretty() {
        let (l, buf) = logger(Level::Info, Format::Pretty);
        l.info("login", &[("api_key", json!("abcdefgh"))]);
        let out = text(&buf);
        assert!(!out.contains("abcdefgh"));
        assert!(out.contains("api_key=a******h"));
    }

    #[test]
    fn reserved_field_names_cannot_be_overridden() {
        let (l, buf) = logger(Level::Info, Format::Json);
        l.info("real message", &[("message", json!("spoofed"))]);
        assert_eq!(last_json(&buf)["message"], "real message");
    }

    #[test]
    fn success_level_emits_suc_badge() {
        let (l, buf) = logger(Level::Trace, Format::Pretty);
        l.success("operation complete", &[]);
        let out = text(&buf);
        assert!(out.contains("SUC"));
        assert!(out.contains("operation complete"));
    }

    #[test]
    fn success_level_emits_correct_json_level() {
        let (l, buf) = logger(Level::Trace, Format::Json);
        l.success("all good", &[("count", json!(7))]);
        let data = last_json(&buf);
        assert_eq!(data["level"], "success");
        assert_eq!(data["message"], "all good");
        assert_eq!(data["count"], 7);
    }

    #[test]
    fn success_is_filtered_below_warn() {
        let (l, buf) = logger(Level::Warn, Format::Json);
        l.success("should be dropped", &[]);
        assert!(text(&buf).is_empty());
    }

    #[test]
    fn trace_id_propagates_to_all_records() {
        let (l, buf) = logger(Level::Info, Format::Json);
        let tl = l.with_trace_id("trace-abc-123");
        tl.info("first", &[]);
        tl.warn("second", &[]);
        let out = text(&buf);
        for line in out.trim().lines() {
            let record = from_json_str(line).unwrap();
            assert_eq!(record["trace_id"], "trace-abc-123");
        }
    }

    #[test]
    #[cfg(feature = "uuid")]
    fn with_new_trace_id_generates_unique_ids() {
        let (base, buf1) = logger(Level::Info, Format::Json);
        let buf2 = Arc::new(Mutex::new(Vec::<u8>::new()));
        let config = Config::default()
            .with_level(Level::Info)
            .with_format(Format::Json)
            .with_no_color(true);
        let base2 = Logger::with_writer(config, Box::new(SharedBuf(buf2.clone())));

        base.with_new_trace_id().info("a", &[]);
        base2.with_new_trace_id().info("b", &[]);

        let id1 = last_json(&buf1)["trace_id"].as_str().unwrap().to_string();
        let id2 = last_json(&buf2)["trace_id"].as_str().unwrap().to_string();

        // Both must be non-empty and distinct.
        assert!(!id1.is_empty(), "trace_id must be set");
        assert_ne!(id1, id2, "each call must produce a unique UUIDv7");
    }
}
