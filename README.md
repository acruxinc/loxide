# loxide

**A fast, dependency-free structured logging library for Rust.**

[![CI](https://github.com/acruxinc/loxide/actions/workflows/ci.yml/badge.svg)](https://github.com/acruxinc/loxide/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/loxide.svg)](https://crates.io/crates/loxide)
[![Documentation](https://docs.rs/loxide/badge.svg)](https://docs.rs/loxide)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

loxide is a small, batteries-included logger for applications and services. It
produces **colored, human-readable logs** in development and **compact JSON**
in production — automatically — with scoped context, sensitive-field redaction,
and ergonomic macros. It has **zero third-party dependencies**: its JSON value
type and UTC time formatting are built entirely on the standard library, so it
compiles quickly and adds nothing to your dependency tree.

---

## Table of contents

- [Why loxide?](#why-loxide)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Output formats](#output-formats)
- [Log levels](#log-levels)
- [Structured fields](#structured-fields)
- [Sub-loggers (scoped context)](#sub-loggers-scoped-context)
- [Automatic redaction](#automatic-redaction)
- [Convenience helpers](#convenience-helpers)
- [Configuration](#configuration)
- [Writing to a custom sink](#writing-to-a-custom-sink)
- [Performance & thread-safety](#performance--thread-safety)
- [MSRV](#msrv)
- [License](#license)

## Why loxide?

- **Zero dependencies.** No `serde`, no `chrono` — nothing. Faster builds, smaller
  supply chain, fewer CVEs to track.
- **Two formats, one API.** Pretty for humans, JSON for machines. `Format::Auto`
  picks the right one based on whether stderr is a terminal.
- **Structured by default.** Every record carries typed key/value fields.
- **Safe by default.** Secrets such as `password` and `api_key` are redacted in
  *both* formats, so they never leak into your logs.
- **Ergonomic.** `log_info!(logger, "msg", "k" => v)` macros capture the call
  site automatically; sub-loggers attach context once.
- **Thread-safe & cheap to clone.** Share one logger across your whole app.

## Installation

```toml
[dependencies]
loxide = "0.0.3"
```

Or from the command line:

```sh
cargo add loxide
```

## Quick start

```rust
use loxide::{Config, Level, Logger, json, log_info};

fn main() {
    // In real apps, `Config::from_env()` is usually what you want.
    let logger = Logger::new(Config::default().with_level(Level::Debug));

    // Method style — fields are `(name, JsonValue)` pairs.
    logger.info("server started", &[("port", json!(8080)), ("tls", json!(true))]);

    // Macro style — `key => value` fields, with automatic caller capture.
    log_info!(logger, "user logged in", "user" => "ada", "attempt" => 1);
}
```

## Output formats

`Format::Auto` (the default) renders **pretty** output when stderr is a terminal
and **JSON** otherwise:

**Pretty**

```text
2026/07/12 10:11:12 UTC INF server started port=8080 tls=true
```

**JSON**

```json
{"time":"2026-07-12T10:11:12Z","level":"info","message":"server started","port":8080,"tls":true}
```

Force a format explicitly with `Config::default().with_format(Format::Json)`.

## Log levels

From least to most severe: `Trace`, `Debug`, `Info`, `Warn`, `Error`, `Fatal`.
A record is emitted only when its level is at least the configured minimum, so
setting the logger to `Warn` silently drops `Info` and below.

```rust
use loxide::{Config, Level, Logger};

let logger = Logger::new(Config::default().with_level(Level::Warn));
logger.info("ignored", &[]);   // dropped
logger.error("shown", &[]);    // emitted
```

Each level has a shortcut method (`logger.info(...)`) and a call-site-capturing
macro (`log_info!`, `log_warn!`, …).

## Structured fields

Field values accept anything convertible into a `JsonValue` (strings, integers,
floats, bools, `Option`, `Vec`, …) via the `json!` macro:

```rust
use loxide::{Config, Logger, log_info};

let logger = Logger::new(Config::default());
log_info!(logger, "request completed",
    "method" => "GET",
    "status" => 200,
    "duration_ms" => 12.5,
    "cached" => true,
);
```

An `error` field is highlighted in red in pretty output.

## Sub-loggers (scoped context)

Attach context once; it appears on every record the sub-logger emits. Sub-loggers
share the parent's output sink, so they are cheap to create.

```rust
use loxide::{Config, Logger, json};

let logger = Logger::new(Config::default());
let request = logger.with_component("api").with_request_id("req-7");

request.info("handling request", &[("path", json!("/health"))]);
// => ... component=api request_id=req-7 path=/health
```

`with_component`, `with_request_id`, and `with_trace_id` are shorthands for the
general `with_field(key, value)`.

## Automatic redaction

Fields whose key looks sensitive are masked automatically — in both pretty and
JSON output:

```rust
use loxide::{Config, Logger, json};

let logger = Logger::new(Config::default());
logger.info("login", &[
    ("username", json!("ada")),
    ("password", json!("super_secret_pass")),
]);
// password is rendered as "s**************s", never in the clear
```

Detected substrings include `password`, `secret`, `token`, `authorization`,
`api_key`, `private_key`, `credential`, `credit_card`, `cvv`, `ssn`, and more.
You can also scrub a map yourself with `redact_map`, or check a key with
`is_sensitive_key`.

## Convenience helpers

The `helpers` module provides ready-made loggers for recurring events, with
consistent field names and levels:

```rust
use loxide::{Config, Logger, log_request, log_response, log_db_query};

let logger = Logger::new(Config::default());
log_request(&logger, "GET", "/v1/users/42", "ada");
log_response(&logger, "GET", "/v1/users/42", 200, 12.0, None); // level from status
log_db_query(&logger, "SELECT", "users", 3.0, 42);
```

`log_response` chooses the level from the status code (`5xx` → error, `4xx` →
warn, else info). See also `log_service_error`, `log_service_debug`, and the
`log_success!` macro (or `Logger::success`) for positive milestones.

## Configuration

Build a `Config` directly, tweak it with the `with_*` builder methods, or read
the environment:

```rust
use loxide::{Config, Logger};

let logger = Logger::new(Config::from_env());
```

| Variable     | Effect                                              |
|--------------|-----------------------------------------------------|
| `LOG_LEVEL`  | Minimum level (`trace`..`fatal`; aliases accepted). |
| `LOG_FORMAT` | `auto`, `pretty`, or `json`.                        |
| `LOG_CALLER` | `1` or `true` enables `file:line` in records.       |
| `NO_COLOR`   | If set (any value), disables ANSI colors.           |
| `TERM`       | `dumb` disables colors.                             |

## Writing to a custom sink

Any `Box<dyn Write + Send>` works — files, buffers, sockets:

```rust
use loxide::{Config, Format, Logger};

let file = std::fs::File::create("app.log").unwrap();
let logger = Logger::with_writer(
    Config::default().with_format(Format::Json),
    Box::new(file),
);
logger.info("written to file", &[]);
```

`Logger::new` writes to stderr and `Logger::stdout` writes to stdout.

## Performance & thread-safety

`Logger` is `Clone` and `Send + Sync`. The output sink is shared behind an
`Arc<Mutex<…>>`, and each record is rendered to a single string and written with
one locked `writeln!`, so concurrent records never interleave. Sharing one logger
(and its sub-loggers) across threads is the intended usage.

Use `logger.enabled(level)` to guard expensive field computation:

```rust
# use loxide::{Config, Level, Logger, json};
# let logger = Logger::new(Config::default());
if logger.enabled(Level::Debug) {
    logger.debug("expensive", &[("data", json!(/* compute */ 1))]);
}
```

## MSRV

loxide requires **Rust 1.88** or newer (edition 2024).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.
