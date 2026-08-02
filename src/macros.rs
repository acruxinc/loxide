//! Ergonomic logging macros that capture the call site automatically.
//!
//! Each macro mirrors a [`Level`](crate::Level) and records the source file and
//! line via [`file!`] / [`line!`], so caller information is correct even though
//! the actual logging happens inside the library. Whether that location is
//! printed still depends on [`Config::caller`](crate::Config::caller).
//!
//! Fields use `key => value` syntax and accept anything convertible into a
//! [`JsonValue`](crate::JsonValue):
//!
//! ```
//! use loxide::{Config, Logger, log_info};
//!
//! let logger = Logger::new(Config::default());
//! log_info!(logger, "user logged in", "user" => "ada", "attempt" => 1);
//! ```

/// Internal helper: expands a level plus `key => value` pairs into a call to
/// [`Logger::log_with_caller`](crate::Logger::log_with_caller).
#[doc(hidden)]
#[macro_export]
macro_rules! __loxide_log {
    ($logger:expr, $level:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {
        $logger.log_with_caller(
            $level,
            $msg,
            &[$(($key, $crate::json!($val)),)*],
            file!(),
            line!(),
        )
    };
}

/// Logs at [`Level::Trace`](crate::Level::Trace) with caller info.
#[macro_export]
macro_rules! log_trace {
    ($logger:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {
        $crate::__loxide_log!($logger, $crate::Level::Trace, $msg $(, $key => $val)*)
    };
}

/// Logs at [`Level::Debug`](crate::Level::Debug) with caller info.
#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {
        $crate::__loxide_log!($logger, $crate::Level::Debug, $msg $(, $key => $val)*)
    };
}

/// Logs at [`Level::Info`](crate::Level::Info) with caller info.
#[macro_export]
macro_rules! log_info {
    ($logger:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {
        $crate::__loxide_log!($logger, $crate::Level::Info, $msg $(, $key => $val)*)
    };
}

/// Logs at [`Level::Warn`](crate::Level::Warn) with caller info.
#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {
        $crate::__loxide_log!($logger, $crate::Level::Warn, $msg $(, $key => $val)*)
    };
}

/// Logs at [`Level::Error`](crate::Level::Error) with caller info.
#[macro_export]
macro_rules! log_error {
    ($logger:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {
        $crate::__loxide_log!($logger, $crate::Level::Error, $msg $(, $key => $val)*)
    };
}

/// Logs at [`Level::Fatal`](crate::Level::Fatal) with caller info.
#[macro_export]
macro_rules! log_fatal {
    ($logger:expr, $msg:expr $(, $key:expr => $val:expr)* $(,)?) => {
        $crate::__loxide_log!($logger, $crate::Level::Fatal, $msg $(, $key => $val)*)
    };
}
