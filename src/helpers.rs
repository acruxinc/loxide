//! Task-oriented convenience helpers for common logging patterns.
//!
//! These wrap [`Logger`] with sensible field names and levels for recurring
//! situations — HTTP requests and responses, database queries, and service
//! operations — so applications log them consistently.

use crate::config::Level;
use crate::json;
use crate::json::JsonValue;
use crate::logger::Logger;

/// Logs a success message at [`Level::Info`] with `result="success"`.
pub fn log_success(logger: &Logger, msg: &str) {
    logger.info(msg, &[("result", json!("success"))]);
}

/// Logs an incoming request (HTTP, gRPC, …) at [`Level::Info`].
pub fn log_request(logger: &Logger, method: &str, path: &str, user: &str) {
    logger.info(
        "Request received",
        &[
            ("method", json!(method)),
            ("path", json!(path)),
            ("user", json!(user)),
        ],
    );
}

/// Logs an outgoing response, selecting the level from the status code:
/// `5xx` → [`Level::Error`], `4xx` → [`Level::Warn`], everything else
/// → [`Level::Info`]. When `error` is `Some`, it is attached as an `error`
/// field.
pub fn log_response(
    logger: &Logger,
    method: &str,
    path: &str,
    status: u16,
    duration_ms: f64,
    error: Option<&str>,
) {
    let mut fields = vec![
        ("method", json!(method)),
        ("path", json!(path)),
        ("status", json!(status)),
        ("duration_ms", json!(duration_ms)),
    ];
    if let Some(err) = error {
        fields.push(("error", json!(err)));
    }

    let level = match status {
        500..=599 => Level::Error,
        400..=499 => Level::Warn,
        _ => Level::Info,
    };
    logger.log(level, "Response sent", &fields);
}

/// Logs a database query at [`Level::Debug`] with timing and row count.
pub fn log_db_query(
    logger: &Logger,
    operation: &str,
    table: &str,
    duration_ms: f64,
    rows_affected: i64,
) {
    logger.log(
        Level::Debug,
        "Database query executed",
        &[
            ("operation", json!(operation)),
            ("table", json!(table)),
            ("duration_ms", json!(duration_ms)),
            ("rows_affected", json!(rows_affected)),
        ],
    );
}

/// Logs a service-level error at [`Level::Error`] with `service` and
/// `operation` context, plus any additional `fields`.
pub fn log_service_error(
    logger: &Logger,
    service: &str,
    operation: &str,
    error: &str,
    fields: &[(&str, JsonValue)],
) {
    let mut all = vec![
        ("error", json!(error)),
        ("service", json!(service)),
        ("operation", json!(operation)),
    ];
    all.extend_from_slice(fields);
    logger.error("Service operation failed", &all);
}

/// Logs service-level debug information at [`Level::Debug`] with `service` and
/// `operation` context, plus any additional `fields`.
pub fn log_service_debug(
    logger: &Logger,
    service: &str,
    operation: &str,
    message: &str,
    fields: &[(&str, JsonValue)],
) {
    let mut all = vec![("service", json!(service)), ("operation", json!(operation))];
    all.extend_from_slice(fields);
    logger.log(Level::Debug, message, &all);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Format};
    use crate::json::{JsonValue, from_json_str};
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn test_logger() -> (Logger, Arc<Mutex<Vec<u8>>>) {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let config = Config::default()
            .with_level(Level::Trace)
            .with_format(Format::Json)
            .with_no_color(true);
        (
            Logger::with_writer(config, Box::new(SharedBuf(buf.clone()))),
            buf,
        )
    }

    fn last_json(buf: &Arc<Mutex<Vec<u8>>>) -> JsonValue {
        let output = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        from_json_str(output.trim().lines().last().unwrap()).unwrap()
    }

    #[test]
    fn success_records_result_field() {
        let (l, buf) = test_logger();
        log_success(&l, "done");
        assert_eq!(last_json(&buf)["result"], "success");
    }

    #[test]
    fn request_records_method_path_user() {
        let (l, buf) = test_logger();
        log_request(&l, "GET", "/api/users", "ada");
        let data = last_json(&buf);
        assert_eq!(data["method"], "GET");
        assert_eq!(data["path"], "/api/users");
        assert_eq!(data["user"], "ada");
        assert_eq!(data["message"], "Request received");
    }

    #[test]
    fn response_level_follows_status() {
        for (status, expected) in [(200u16, "info"), (404, "warn"), (500, "error")] {
            let (l, buf) = test_logger();
            log_response(&l, "GET", "/", status, 10.0, None);
            let data = last_json(&buf);
            assert_eq!(data["level"], expected, "status {status}");
        }
    }

    #[test]
    fn response_attaches_error() {
        let (l, buf) = test_logger();
        log_response(&l, "GET", "/api", 500, 150.0, Some("db timeout"));
        let data = last_json(&buf);
        assert_eq!(data["level"], "error");
        assert_eq!(data["error"], "db timeout");
    }

    #[test]
    fn db_query_records_details() {
        let (l, buf) = test_logger();
        log_db_query(&l, "SELECT", "users", 5.0, 42);
        let data = last_json(&buf);
        assert_eq!(data["operation"], "SELECT");
        assert_eq!(data["table"], "users");
        assert_eq!(data["rows_affected"], 42);
        assert_eq!(data["level"], "debug");
    }

    #[test]
    fn service_error_includes_context() {
        let (l, buf) = test_logger();
        log_service_error(
            &l,
            "UserService",
            "CreateUser",
            "duplicate email",
            &[("email", json!("user@example.com"))],
        );
        let data = last_json(&buf);
        assert_eq!(data["service"], "UserService");
        assert_eq!(data["operation"], "CreateUser");
        assert_eq!(data["level"], "error");
        assert_eq!(data["email"], "user@example.com");
    }

    #[test]
    fn service_debug_includes_context() {
        let (l, buf) = test_logger();
        log_service_debug(
            &l,
            "CacheService",
            "Get",
            "Cache lookup",
            &[("hit", json!(true))],
        );
        let data = last_json(&buf);
        assert_eq!(data["service"], "CacheService");
        assert_eq!(data["level"], "debug");
        assert_eq!(data["hit"], true);
    }
}
