# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.3] - 2026-08-10

### Added
- **UUIDv7 Trace IDs**: Added optional `uuid` feature. Exposes `logger.with_new_trace_id()` to automatically generate and attach time-ordered, uniquely sortable UUIDv7 trace IDs to log records.
- **`log` Integration**: Added optional `log` feature which implements a bridge (`loxide::log_compat`) to seamlessly forward all `log` crate macros (like `log::info!`) into `loxide`. 
- **`tracing` Integration**: Added optional `tracing` feature that provides a custom tracing layer (`loxide::tracing_compat::LoxideLayer`) to sink tracing events and fully retain their structured types (integers, booleans, floats, etc.) into `loxide`.
- **Documentation**: Explicitly documented that `Level::Fatal` acts as a severity hint and does not internally call `std::process::exit` or panic, leaving termination to the caller.
- **Documentation**: Added code comments explaining the deliberate omission of `u64` from internal lossless integer widening traits (to prevent silent overflows into `i64`).

### Changed
- **Performance / Semantics**: Refactored `Logger::fields` to use `Vec<(String, JsonValue)>` instead of `BTreeMap`. This preserves exactly the order of field insertion while maintaining rapid rendering speeds and resolving arbitrary alphabetical reshuffling of logged fields.
- **Config**: Enhanced `Format::Auto` resolution to correctly respect the destination sink. Custom sinks created via `Logger::with_writer` (like files, buffers, or network sockets) will now automatically fall back to JSON format, instead of inaccurately probing `stderr`.
- **API (Redact)**: Made `redact_map` fully generic. It now accepts `HashMap<String, V>` (where `V: Into<JsonValue> + Clone`) and returns a `HashMap<String, JsonValue>`, making it drastically more versatile for scrubbing maps of various data types.
- **Packaging**: Aligned `rust-version` MSRV metadata in `Cargo.toml` to `1.88.0` to properly match documentation.
- **Packaging**: Updated the `license` field in `Cargo.toml` to standard SPDX identifier format `MIT OR Apache-2.0`, properly reflecting the dual-license intent. Added `LICENSE-MIT` and `LICENSE-APACHE` text files.
- **Source**: Cleaned up the source tree by removing empty placeholder modules (`global.rs`, `metrics.rs`, `serde_impl.rs`, `trace.rs`, `writer.rs`) to prevent contributor confusion.

### Fixed
- **Testing**: Fixed an issue in the test-only JSON parser where multi-byte UTF-8 sequences were erroneously truncated by single-byte `char` casting. The parser now correctly extracts multi-byte UTF-8 string values.
- **Linting**: Addressed and resolved `clippy::collapsible_if` warnings in the environment configuration parser.
