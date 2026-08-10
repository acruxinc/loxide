//! loxide — structured logging showcase.
//!
//! Run with: `cargo run --example demo`

use loxide::{
    Config, Format, Level, Logger, log_debug, log_error, log_info, log_success, log_trace, log_warn,
};

fn main() {
    // A pretty, colored logger writing to stdout so the demo is easy to read.
    let logger = Logger::stdout(
        Config::default()
            .with_level(Level::Trace)
            .with_format(Format::Pretty)
            .with_caller(true),
    );

    banner("loxide — Structured Logging Showcase");

    // 1. Basic levels ------------------------------------------------------
    section("Log levels");
    log_trace!(logger, "TRACE — finest granularity");
    log_debug!(logger, "DEBUG — diagnostic detail");
    log_info!(logger, "INFO — normal operation");
    log_success!(logger, "SUCCESS — positive milestone");
    log_warn!(logger, "WARN — something looks off");
    log_error!(logger, "ERROR — something failed");

    // 2. Request lifecycle with trace ID -----------------------------------
    section("Request lifecycle (Trace ID)");

    // We create a sub-logger that attaches `trace_id` to all subsequent logs.
    // This allows you to track an entire request flow across components.
    let request_logger = logger.with_new_trace_id();

    // Simulate HTTP Request Start
    log_info!(
        request_logger,
        "Incoming request",
        "method" => "POST",
        "path" => "/api/v1/todo"
    );

    // Pass the logger down into the service layer
    create_todo_handler(&request_logger, "Buy groceries");

    log_success!(
        request_logger,
        "Request completed",
        "status" => 201,
        "duration_ms" => 42
    );

    // 3. Demonstrating Redaction -------------------------------------------
    section("Sensitive field redaction");
    let auth_logger = logger.with_component("auth_service");
    log_info!(
        auth_logger,
        "User login attempt",
        "username" => "admin",
        "password" => "supersecret123", // Automatically redacted!
        "token" => "eyJh..."            // Automatically redacted!
    );

    // 4. JSON output -------------------------------------------------------
    section("JSON output (production / aggregators)");
    let json_logger = Logger::stdout(
        Config::default()
            .with_level(Level::Info)
            .with_format(Format::Json)
            .with_caller(true),
    );

    // The JSON logger produces cleanly structured lines suitable for Datadog, ELK, etc.
    let json_trace_logger = json_logger.with_new_trace_id();
    log_success!(
        json_trace_logger,
        "Transaction complete",
        "amount" => 1500,
        "currency" => "USD"
    );
}

// --------------------------------------------------------------------------
// Mock Service Layers demonstrating logger passing
// --------------------------------------------------------------------------

fn create_todo_handler(logger: &Logger, task: &str) {
    let service_logger = logger.with_component("todo_service");

    log_debug!(
        service_logger,
        "Starting create_todo_handler",
        "task" => task
    );

    // Pass down to DB layer
    insert_todo_db(&service_logger, task);
}

fn insert_todo_db(logger: &Logger, task: &str) {
    let db_logger = logger.with_component("database");

    log_trace!(
        db_logger,
        "Executing SQL INSERT",
        "query" => "INSERT INTO todos (task) VALUES ($1)",
        "binds" => task
    );

    // Simulate DB success
    log_success!(db_logger, "Todo successfully inserted into database");
}

fn banner(title: &str) {
    println!("\n=== {title} ===\n");
}

fn section(title: &str) {
    println!("\n-- {title} --\n");
}
