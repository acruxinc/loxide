//! Logger configuration: [`Level`], [`Format`], and [`Config`].

use std::env;
use std::fmt;
use std::io::IsTerminal;
use std::str::FromStr;

/// Severity of a log record, ordered from least to most severe.
///
/// Levels are comparable, so filtering is a simple comparison: a record is
/// emitted when its level is greater than or equal to the logger's configured
/// [`Config::level`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Extremely verbose, fine-grained tracing.
    Trace = 0,
    /// Diagnostic detail useful while debugging.
    Debug = 1,
    /// Normal, expected operational messages.
    Info = 2,
    /// A successful operation or positive milestone.
    Success = 3,
    /// Something unexpected that is not (yet) an error.
    Warn = 4,
    /// A failure that needs attention.
    Error = 5,
    /// An unrecoverable failure.
    ///
    /// # Note
    /// `Fatal` is a severity hint only — loxide **does not** call
    /// [`std::process::exit`] or panic on your behalf. It is the caller's
    /// responsibility to terminate the process after logging a fatal event.
    Fatal = 6,
}

impl Level {
    /// Every level, ordered from [`Level::Trace`] to [`Level::Fatal`].
    pub const ALL: [Level; 7] = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Success,
        Level::Warn,
        Level::Error,
        Level::Fatal,
    ];

    /// Returns the fixed-width, three-character badge used in pretty output.
    pub fn badge(self) -> &'static str {
        match self {
            Level::Trace => "TRC",
            Level::Debug => "DBG",
            Level::Info => "INF",
            Level::Success => "SUC",
            Level::Warn => "WAR",
            Level::Error => "ERR",
            Level::Fatal => "FTL",
        }
    }

    /// Returns the lowercase name used in JSON output.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "trace",
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Success => "success",
            Level::Warn => "warn",
            Level::Error => "error",
            Level::Fatal => "fatal",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string cannot be parsed into a [`Level`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLevelError(String);

impl fmt::Display for ParseLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid log level: {:?}", self.0)
    }
}

impl std::error::Error for ParseLevelError {}

impl FromStr for Level {
    type Err = ParseLevelError;

    /// Parses a level name case-insensitively. Accepts common aliases such as
    /// `warning` for [`Level::Warn`] and `critical` for [`Level::Fatal`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "trace" => Ok(Level::Trace),
            "debug" => Ok(Level::Debug),
            "info" => Ok(Level::Info),
            "success" => Ok(Level::Success),
            "warn" | "warning" => Ok(Level::Warn),
            "error" => Ok(Level::Error),
            "fatal" | "critical" => Ok(Level::Fatal),
            _ => Err(ParseLevelError(s.to_string())),
        }
    }
}

/// How log records are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Choose [`Format::Pretty`] when stderr is a terminal, otherwise
    /// [`Format::Json`]. This is the default.
    #[default]
    Auto,
    /// Human-readable, optionally colored, single-line output.
    Pretty,
    /// One compact JSON object per line, ideal for log aggregators.
    Json,
}

/// Error returned when a string cannot be parsed into a [`Format`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFormatError(String);

impl fmt::Display for ParseFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid log format: {:?}", self.0)
    }
}

impl std::error::Error for ParseFormatError {}

impl FromStr for Format {
    type Err = ParseFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Format::Auto),
            "pretty" | "text" | "console" => Ok(Format::Pretty),
            "json" => Ok(Format::Json),
            _ => Err(ParseFormatError(s.to_string())),
        }
    }
}

/// Describes the kind of writer a [`Logger`](crate::logger::Logger) is
/// writing to. Used by [`Config::resolved_format`] to correctly resolve
/// [`Format::Auto`] without always probing `stderr`.
///
/// When you create a logger with `Logger::new` (→ stderr) or `Logger::stdout`
/// (→ stdout) the kind is inferred automatically. If you supply a custom sink
/// via `Logger::with_writer`, the format is resolved eagerly at construction
/// time based on the actual sink you pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WriterKind {
    /// The logger writes to standard error (the default).
    #[default]
    Stderr,
    /// The logger writes to standard output.
    Stdout,
    /// The logger writes to an arbitrary, non-terminal sink (file, buffer, …).
    Other,
}

/// Logger configuration.
///
/// Construct one directly, start from [`Config::default`], or read the process
/// environment with [`Config::from_env`]. The builder-style `with_*` methods
/// make small tweaks ergonomic:
///
/// ```
/// use loxide::{Config, Format, Level};
///
/// let config = Config::default()
///     .with_level(Level::Debug)
///     .with_format(Format::Json)
///     .with_caller(true);
/// assert_eq!(config.level, Level::Debug);
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Minimum level to emit; records below this are dropped.
    pub level: Level,
    /// Output format.
    pub format: Format,
    /// Whether to include the source file and line that emitted the record.
    pub caller: bool,
    /// Disable ANSI colors even when writing to a terminal.
    pub no_color: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            level: Level::Info,
            format: Format::Auto,
            caller: false,
            no_color: false,
        }
    }
}

impl Config {
    /// Builds a [`Config`] from environment variables, falling back to
    /// [`Config::default`] for anything unset or unrecognized:
    ///
    /// | Variable     | Effect                                              |
    /// |--------------|-----------------------------------------------------|
    /// | `LOG_LEVEL`  | Minimum level (`trace`..`fatal`, aliases accepted). |
    /// | `LOG_FORMAT` | `auto`, `pretty`, or `json`.                        |
    /// | `LOG_CALLER` | `1` or `true` enables caller info.                  |
    /// | `NO_COLOR`   | If set (any value), disables colors.                |
    /// | `TERM`       | `dumb` disables colors.                             |
    pub fn from_env() -> Self {
        let mut config = Config::default();

        if let Some(parsed) = env::var("LOG_LEVEL").ok().and_then(|l| l.parse().ok()) {
            config.level = parsed;
        }

        if let Some(parsed) = env::var("LOG_FORMAT").ok().and_then(|l| l.parse().ok()) {
            config.format = parsed;
        }

        if let Ok(caller) = env::var("LOG_CALLER") {
            config.caller = matches!(caller.as_str(), "1" | "true");
        }

        if env::var_os("NO_COLOR").is_some() || env::var("TERM").as_deref() == Ok("dumb") {
            config.no_color = true;
        }

        config
    }

    /// Returns a copy with the minimum [`Level`] set.
    #[must_use]
    pub fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Returns a copy with the output [`Format`] set.
    #[must_use]
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Returns a copy with caller information enabled or disabled.
    #[must_use]
    pub fn with_caller(mut self, caller: bool) -> Self {
        self.caller = caller;
        self
    }

    /// Returns a copy with ANSI colors forcibly disabled or re-enabled.
    #[must_use]
    pub fn with_no_color(mut self, no_color: bool) -> Self {
        self.no_color = no_color;
        self
    }

    /// Resolves [`Format::Auto`] to a concrete [`Format`] based on the kind
    /// of writer the logger is attached to. [`Format::Pretty`] and
    /// [`Format::Json`] are returned unchanged regardless of `writer_kind`.
    ///
    /// - `WriterKind::Stderr` → pretty when stderr is a terminal, else JSON.
    /// - `WriterKind::Stdout` → pretty when stdout is a terminal, else JSON.
    /// - `WriterKind::Other` → always JSON (files, buffers, sockets are never
    ///   terminals).
    ///
    /// This avoids the previous bug where a logger writing to a file would
    /// check whether *stderr* was a terminal and potentially emit pretty output
    /// into a structured log file.
    pub fn resolved_format(&self, writer_kind: WriterKind) -> Format {
        match self.format {
            Format::Auto => match writer_kind {
                WriterKind::Stderr => {
                    if std::io::stderr().is_terminal() {
                        Format::Pretty
                    } else {
                        Format::Json
                    }
                }
                WriterKind::Stdout => {
                    if std::io::stdout().is_terminal() {
                        Format::Pretty
                    } else {
                        Format::Json
                    }
                }
                WriterKind::Other => Format::Json,
            },
            explicit => explicit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering_enables_filtering() {
        assert!(Level::Trace < Level::Info);
        assert!(Level::Info < Level::Success);
        assert!(Level::Success < Level::Warn);
        assert!(Level::Error < Level::Fatal);
        assert!(Level::Info >= Level::Info);
    }

    #[test]
    fn level_parsing_and_aliases() {
        assert_eq!("info".parse(), Ok(Level::Info));
        assert_eq!("success".parse(), Ok(Level::Success));
        assert_eq!("WARNING".parse(), Ok(Level::Warn));
        assert_eq!("Critical".parse(), Ok(Level::Fatal));
        assert!("nonsense".parse::<Level>().is_err());
    }

    #[test]
    fn success_badge_and_str() {
        assert_eq!(Level::Success.badge(), "SUC");
        assert_eq!(Level::Success.as_str(), "success");
    }

    #[test]
    fn format_parsing() {
        assert_eq!("json".parse(), Ok(Format::Json));
        assert_eq!("pretty".parse(), Ok(Format::Pretty));
        assert_eq!("auto".parse(), Ok(Format::Auto));
        assert!("xml".parse::<Format>().is_err());
    }

    #[test]
    fn builder_methods_compose() {
        let config = Config::default()
            .with_level(Level::Trace)
            .with_format(Format::Json)
            .with_caller(true)
            .with_no_color(true);
        assert_eq!(config.level, Level::Trace);
        assert_eq!(config.format, Format::Json);
        assert!(config.caller);
        assert!(config.no_color);
    }

    #[test]
    fn explicit_formats_resolve_to_themselves() {
        // Explicit formats are always returned unchanged, regardless of the sink.
        for kind in [WriterKind::Stderr, WriterKind::Stdout, WriterKind::Other] {
            assert_eq!(
                Config::default()
                    .with_format(Format::Json)
                    .resolved_format(kind),
                Format::Json
            );
            assert_eq!(
                Config::default()
                    .with_format(Format::Pretty)
                    .resolved_format(kind),
                Format::Pretty
            );
        }
    }

    #[test]
    fn auto_resolves_to_json_for_non_terminal_sinks() {
        // WriterKind::Other (file, buffer, socket) is never a terminal.
        assert_eq!(
            Config::default().resolved_format(WriterKind::Other),
            Format::Json
        );
    }
}
