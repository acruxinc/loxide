//! Bridge to the `tracing` crate.

use crate::{Level, Logger, json::JsonValue};
use tracing_core::{Event, subscriber::Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// A `tracing` layer that forwards events to `loxide`.
pub struct LoxideLayer {
    logger: Logger,
}

impl LoxideLayer {
    /// Creates a new layer forwarding to the given logger.
    pub fn new(logger: Logger) -> Self {
        Self { logger }
    }
}

impl<S: Subscriber> Layer<S> for LoxideLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = match *meta.level() {
            tracing_core::Level::ERROR => Level::Error,
            tracing_core::Level::WARN => Level::Warn,
            tracing_core::Level::INFO => Level::Info,
            tracing_core::Level::DEBUG => Level::Debug,
            tracing_core::Level::TRACE => Level::Trace,
        };

        if !self.logger.enabled(level) {
            return;
        }

        struct Visitor {
            message: String,
            fields: Vec<(String, JsonValue)>,
        }

        impl tracing_core::field::Visit for Visitor {
            fn record_f64(&mut self, field: &tracing_core::Field, value: f64) {
                if field.name() == "message" {
                    self.message = value.to_string();
                } else {
                    self.fields
                        .push((field.name().to_string(), JsonValue::from(value)));
                }
            }

            fn record_i64(&mut self, field: &tracing_core::Field, value: i64) {
                if field.name() == "message" {
                    self.message = value.to_string();
                } else {
                    self.fields
                        .push((field.name().to_string(), JsonValue::from(value)));
                }
            }

            fn record_u64(&mut self, field: &tracing_core::Field, value: u64) {
                if field.name() == "message" {
                    self.message = value.to_string();
                } else if value <= i64::MAX as u64 {
                    self.fields
                        .push((field.name().to_string(), JsonValue::from(value as i64)));
                } else {
                    self.fields
                        .push((field.name().to_string(), JsonValue::from(value.to_string())));
                }
            }

            fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
                if field.name() == "message" {
                    self.message = value.to_string();
                } else {
                    self.fields
                        .push((field.name().to_string(), JsonValue::from(value)));
                }
            }

            fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
                if field.name() == "message" {
                    self.message = value.to_string();
                } else {
                    self.fields
                        .push((field.name().to_string(), JsonValue::from(value)));
                }
            }

            fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = format!("{:?}", value);
                } else {
                    self.fields.push((
                        field.name().to_string(),
                        JsonValue::from(format!("{:?}", value)),
                    ));
                }
            }
        }

        let mut visitor = Visitor {
            message: String::new(),
            fields: vec![("target".to_string(), JsonValue::from(meta.target()))],
        };

        event.record(&mut visitor);

        let fields_ref: Vec<(&str, JsonValue)> = visitor
            .fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        if let (Some(file), Some(line)) = (meta.file(), meta.line()) {
            self.logger
                .log_with_caller(level, &visitor.message, &fields_ref, file, line);
        } else {
            self.logger.log(level, &visitor.message, &fields_ref);
        }
    }
}
