//! # loxide
//!
//! A fast, **dependency-free** structured logging library for Rust.
//!
//! loxide gives applications leveled, structured logs with two rendering modes —
//! a colored human-readable format for the console and compact JSON for log
//! aggregators — plus scoped sub-loggers, automatic redaction of sensitive
//! fields, and ergonomic macros. It has **no third-party dependencies**: its own
//! JSON value type ([`JsonValue`]) and UTC time handling ([`time`]) are built on
//! `std` alone, so it compiles fast and adds nothing to your dependency tree.
//!
//! ## Quick start
//!
//! ```
//! use loxide::{Config, Level, Logger, json, log_info};
//!
//! // Build a logger. `Config::from_env()` is the usual choice in real apps.
//! let logger = Logger::new(Config::default().with_level(Level::Debug));
//!
//! // Method style: pass fields as `(name, JsonValue)` pairs.
//! logger.info("server started", &[("port", json!(8080)), ("tls", json!(true))]);
//!
//! // Macro style: `key => value` fields, with automatic caller capture.
//! log_info!(logger, "user logged in", "user" => "ada", "attempt" => 1);
//! ```
//!
//! ## Output formats
//!
//! With [`Format::Auto`] (the default) loxide renders [`Format::Pretty`] when
//! stderr is a terminal and [`Format::Json`] otherwise, so you get readable logs
//! in development and machine-parseable logs in production automatically.
//!
//! - **Pretty:** `2026/07/12 10:11:12 UTC INF server started port=8080 tls=true`
//! - **JSON:** `{"time":"2026-07-12T10:11:12Z","level":"info","message":"server started","port":8080,"tls":true}`
//!
//! ## Sub-loggers
//!
//! Attach context once and have it appear on every subsequent record:
//!
//! ```
//! use loxide::{Config, Logger, json};
//!
//! let logger = Logger::new(Config::default());
//! let request = logger.with_component("api").with_request_id("req-7");
//! request.info("handling", &[("path", json!("/health"))]);
//! // => ... component=api request_id=req-7 path=/health
//! ```
//!
//! ## Redaction
//!
//! Fields whose key looks sensitive (`password`, `api_key`, `authorization`, …)
//! are masked automatically in **both** formats — see the [`redact`] module.
//!
//! ## Configuration via the environment
//!
//! [`Config::from_env`] reads `LOG_LEVEL`, `LOG_FORMAT`, `LOG_CALLER`,
//! `NO_COLOR`, and `TERM`.
//!
//! ## Convenience helpers
//!
//! The [`helpers`] module offers ready-made loggers for common events such as
//! [`log_request`], [`log_response`], and [`log_db_query`].

#![doc(html_root_url = "https://docs.rs/loxide")]
#![forbid(unsafe_code)]

pub mod config;
pub mod helpers;
pub mod json;
pub mod logger;
pub mod time;

pub mod redact;

mod macros;
mod pretty;

pub use config::{Config, Format, Level, ParseFormatError, ParseLevelError};
pub use helpers::{
    log_db_query, log_request, log_response, log_service_debug, log_service_error, log_success,
};
pub use json::{JsonNumber, JsonValue};
pub use logger::Logger;
pub use redact::{is_sensitive_key, redact_map, redact_value};

/// The most commonly used items, re-exported for a single glob import.
///
/// ```
/// use loxide::prelude::*;
///
/// let logger = Logger::new(Config::default());
/// log_info!(logger, "ready", "component" => "boot");
/// ```
pub mod prelude {
    pub use crate::{Config, Format, JsonValue, Level, Logger};
    pub use crate::{json, log_debug, log_error, log_fatal, log_info, log_trace, log_warn};
}
