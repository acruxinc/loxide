//! Human-readable, optionally colored, single-line formatting helpers.
//!
//! These are crate-internal building blocks used by [`crate::Logger`] when it
//! renders in [`crate::Format::Pretty`]. Field values arrive here already
//! redacted, so this module is purely concerned with layout and color.

use crate::config::Level;

// ANSI SGR escape codes.
const RESET: &str = "\x1b[0m";
const GRAY: &str = "\x1b[90m";
const PURPLE: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const BOLD: &str = "\x1b[1m";

/// Wraps `text` in the given ANSI code(s), unless `no_color` is set.
pub(crate) fn colorize(code: &str, text: &str, no_color: bool) -> String {
    if no_color {
        text.to_string()
    } else {
        format!("{code}{text}{RESET}")
    }
}

/// Formats a level as its colored three-character badge. Fatal is emphasized
/// with bold in addition to red.
pub(crate) fn format_level(level: Level, no_color: bool) -> String {
    let badge = level.badge();
    if no_color {
        return badge.to_string();
    }
    let color = match level {
        Level::Trace => GRAY,
        Level::Debug => PURPLE,
        Level::Info => CYAN,
        Level::Warn => YELLOW,
        Level::Error | Level::Fatal => RED,
    };
    let emphasis = if level == Level::Fatal { BOLD } else { "" };
    format!("{color}{emphasis}{badge}{RESET}")
}

/// Formats a timestamp in muted gray.
pub(crate) fn format_timestamp(ts: &str, no_color: bool) -> String {
    colorize(GRAY, ts, no_color)
}

/// Formats a caller location in gray parentheses, e.g. `(server/main.rs:42)`.
pub(crate) fn format_caller(caller: &str, no_color: bool) -> String {
    colorize(GRAY, &format!("({caller})"), no_color)
}

/// Formats a `key=value` pair (gray key, green value).
pub(crate) fn format_field(key: &str, value: &str, no_color: bool) -> String {
    format!(
        "{}{}",
        colorize(GRAY, &format!("{key}="), no_color),
        colorize(GREEN, value, no_color),
    )
}

/// Formats the special `error` field with a red value to make failures pop.
pub(crate) fn format_error_field(value: &str, no_color: bool) -> String {
    format!(
        "{}{}",
        colorize(GRAY, "error=", no_color),
        colorize(RED, value, no_color),
    )
}

/// Shortens a full source path to its last two segments plus line number, e.g.
/// `/home/user/app/server/main.rs` + `42` → `server/main.rs:42`.
pub(crate) fn short_caller(file: &str, line: u32) -> String {
    let mut segments = file.rsplit('/');
    let last = segments.next().unwrap_or(file);
    match segments.next() {
        Some(parent) => format!("{parent}/{last}:{line}"),
        None => format!("{last}:{line}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badges_without_color() {
        assert_eq!(format_level(Level::Info, true), "INF");
        assert_eq!(format_level(Level::Warn, true), "WAR");
        assert_eq!(format_level(Level::Fatal, true), "FTL");
    }

    #[test]
    fn badges_with_color_wrap_in_ansi() {
        let info = format_level(Level::Info, false);
        assert!(info.starts_with("\x1b["));
        assert!(info.contains("INF"));
        assert!(info.ends_with(RESET));

        // Fatal gets both red and bold.
        let fatal = format_level(Level::Fatal, false);
        assert!(fatal.contains(RED));
        assert!(fatal.contains(BOLD));
    }

    #[test]
    fn shortens_caller_paths() {
        assert_eq!(short_caller("server/main.rs", 42), "server/main.rs:42");
        assert_eq!(
            short_caller("/home/user/project/server/main.rs", 42),
            "server/main.rs:42"
        );
        assert_eq!(short_caller("main.rs", 7), "main.rs:7");
    }

    #[test]
    fn fields_render_key_and_value() {
        assert_eq!(format_field("port", "8080", true), "port=8080");
        assert_eq!(format_error_field("boom", true), "error=boom");
    }
}
