//! Dependency-free UTC timestamps.
//!
//! loxide avoids pulling in `chrono` just to stamp log lines. This module reads
//! the wall clock with [`std::time::SystemTime`] and converts the Unix epoch
//! offset into civil (year/month/day) components using Howard Hinnant's
//! well-known `days_from_civil` inverse algorithm, which is exact for all dates
//! the Gregorian calendar covers.

use std::time::{SystemTime, UNIX_EPOCH};

/// A UTC timestamp broken down into calendar and clock components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDateTime {
    /// Full year (e.g. `2026`).
    pub year: i64,
    /// Month of year, `1..=12`.
    pub month: u32,
    /// Day of month, `1..=31`.
    pub day: u32,
    /// Hour of day, `0..=23`.
    pub hour: u32,
    /// Minute of hour, `0..=59`.
    pub minute: u32,
    /// Second of minute, `0..=59`.
    pub second: u32,
}

impl UtcDateTime {
    /// Captures the current wall-clock time as UTC.
    ///
    /// If the system clock is somehow set before the Unix epoch, this falls back
    /// to the epoch itself rather than panicking.
    pub fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self::from_unix_seconds(secs)
    }

    /// Builds a [`UtcDateTime`] from seconds elapsed since the Unix epoch.
    pub fn from_unix_seconds(secs: u64) -> Self {
        let days = (secs / 86_400) as i64;
        let rem = (secs % 86_400) as u32;
        let (year, month, day) = civil_from_days(days);
        UtcDateTime {
            year,
            month,
            day,
            hour: rem / 3_600,
            minute: (rem % 3_600) / 60,
            second: rem % 60,
        }
    }

    /// Formats as ISO 8601 / RFC 3339 in UTC: `YYYY-MM-DDTHH:MM:SSZ`.
    ///
    /// This is the format used for JSON log output.
    pub fn to_iso8601(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Formats for human-friendly console output: `YYYY/MM/DD HH:MM:SS UTC`.
    pub fn to_pretty(&self) -> String {
        format!(
            "{:04}/{:02}/{:02} {:02}:{:02}:{:02} UTC",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Converts days since the Unix epoch (1970-01-01) to `(year, month, day)`.
///
/// Ported from Howard Hinnant's `civil_from_days`
/// (<https://howardhinnant.github.io/date_algorithms.html>).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01 so leap days fall at the end of the cycle.
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // day of era, 0..=146_096
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // 0..=399
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year, 0..=365
    let mp = (5 * doy + 2) / 153; // month index, 0..=11 (March = 0)
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // 1..=31
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // 1..=12
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn known_dates_convert_correctly() {
        assert_eq!(civil_from_days(19_782), (2024, 2, 29)); // leap day
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
    }

    #[test]
    fn decodes_a_full_timestamp() {
        // 2021-01-01T00:00:00Z == 1_609_459_200 seconds since the epoch.
        let dt = UtcDateTime::from_unix_seconds(1_609_459_200);
        assert_eq!(
            dt,
            UtcDateTime {
                year: 2021,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0
            }
        );
        // A few seconds past midnight to exercise the clock components.
        let dt = UtcDateTime::from_unix_seconds(1_609_459_200 + 3_661);
        assert_eq!((dt.hour, dt.minute, dt.second), (1, 1, 1));
    }

    #[test]
    fn iso_format_shape() {
        let ts = UtcDateTime::now().to_iso8601();
        assert!(ts.ends_with('Z'), "expected Z suffix: {ts}");
        assert!(ts.contains('T'), "expected T separator: {ts}");
        assert_eq!(ts.len(), 20, "expected length 20: {ts}");
    }

    #[test]
    fn pretty_format_shape() {
        let ts = UtcDateTime::now().to_pretty();
        assert!(ts.ends_with(" UTC"), "expected UTC suffix: {ts}");
        assert_eq!(ts.len(), 23, "expected length 23: {ts}");
    }
}
