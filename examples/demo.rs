//! loxide — structured logging showcase.
//!
//! Run with: `cargo run --example demo`

use loxide::{
    Config, Format, Level, Logger, json, log_db_query, log_debug, log_error, log_info, log_request,
    log_response, log_service_debug, log_service_error, log_success, log_trace, log_warn,
};

fn main() {
    // A pretty, colored logger writing to stdout so the demo is easy to read.
    let pretty = Logger::stdout(
        Config::default()
            .with_level(Level::Trace)
            .with_format(Format::Pretty)
            .with_caller(true),
    );

    banner("loxide — Structured Logging Showcase");

    // 1. Basic levels ------------------------------------------------------
    section("Log levels");
    log_trace!(pretty, "TRACE — finest granularity");
    log_debug!(pretty, "DEBUG — diagnostic detail");
    log_info!(pretty, "INFO — normal operation");
    log_warn!(pretty, "WARN — something looks off");
    log_error!(pretty, "ERROR — something failed");

    // 2. Structured fields -------------------------------------------------
    section("Structured fields");
    log_info!(pretty, "authentication successful",
        "user" => "ada",
        "action" => "login",
        "ip" => "192.168.1.42",
        "attempt" => 1,
        "mfa" => true,
    );

    // 3. Error context -----------------------------------------------------
    section("Error logging");
    log_error!(pretty, "failed to connect to database",
        "error" => "connection refused: dial tcp 10.0.0.5:5432",
        "host" => "10.0.0.5",
        "port" => 5432,
    );

    // 4. Sub-loggers -------------------------------------------------------
    section("Sub-loggers (component / request scoped)");
    let db = pretty.with_component("database");
    db.info(
        "connection pool initialized",
        &[("driver", json!("pgx")), ("pool_size", json!(25))],
    );

    let request = pretty.with_request_id("req-7f3a-4b2c");
    request.info(
        "processing request",
        &[("method", json!("POST")), ("path", json!("/v1/entries"))],
    );

    // 5. Helper functions --------------------------------------------------
    section("Helper functions");
    log_success(&pretty, "migration completed");
    log_request(&pretty, "GET", "/v1/users/42", "ada");
    log_response(&pretty, "GET", "/v1/users/42", 200, 12.0, None);
    log_response(
        &pretty,
        "GET",
        "/v1/secrets",
        403,
        2.0,
        Some("insufficient permissions"),
    );
    log_response(
        &pretty,
        "POST",
        "/v1/entries",
        500,
        3000.0,
        Some("deadlock detected"),
    );
    log_db_query(&pretty, "SELECT", "users", 3.0, 42);
    log_service_error(
        &pretty,
        "UserService",
        "CreateUser",
        "duplicate email",
        &[("email", json!("user@example.com"))],
    );
    log_service_debug(
        &pretty,
        "CacheService",
        "Get",
        "cache lookup completed",
        &[("key", json!("user:42:profile")), ("hit", json!(true))],
    );

    // 6. Redaction ---------------------------------------------------------
    section("Sensitive field redaction");
    pretty.info(
        "login attempt",
        &[
            ("username", json!("ada")),
            ("password", json!("super_secret_pass_123")),
            ("api_key", json!("sk-proj-abc123def456")),
        ],
    );

    // 7. JSON output -------------------------------------------------------
    section("JSON output (production / aggregators)");
    let json_logger = Logger::stdout(
        Config::default()
            .with_level(Level::Info)
            .with_format(Format::Json)
            .with_caller(true),
    );
    log_info!(json_logger, "application started", "environment" => "production", "version" => "2.4.1");
    log_error!(json_logger, "cache unavailable",
        "error" => "connection timeout",
        "service" => "redis",
        "password" => "should-be-redacted",
    );

    // 8. Environment-based init -------------------------------------------
    section("Environment-based init");
    println!("  (set LOG_LEVEL, LOG_FORMAT, LOG_CALLER, NO_COLOR, TERM to configure)");
    let env_logger = Logger::new(Config::from_env());
    env_logger.info("logger initialized from environment", &[]);
}

fn banner(title: &str) {
    println!("\n=== {title} ===\n");
}

fn section(title: &str) {
    println!("\n-- {title} --\n");
}
