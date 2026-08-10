//! Bridge to the `log` crate.

use crate::{Level, Logger, json::JsonValue};

struct LogBridge {
    logger: Logger,
}

impl log::Log for LogBridge {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.logger.enabled(convert_level(metadata.level()))
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = convert_level(record.level());
            let msg = record.args().to_string();
            let target = record.target();

            if let (Some(file), Some(line)) = (record.file(), record.line()) {
                self.logger.log_with_caller(
                    level,
                    &msg,
                    &[("target", JsonValue::from(target))],
                    file,
                    line,
                );
            } else {
                self.logger
                    .log(level, &msg, &[("target", JsonValue::from(target))]);
            }
        }
    }

    fn flush(&self) {}
}

fn convert_level(level: log::Level) -> Level {
    match level {
        log::Level::Error => Level::Error,
        log::Level::Warn => Level::Warn,
        log::Level::Info => Level::Info,
        log::Level::Debug => Level::Debug,
        log::Level::Trace => Level::Trace,
    }
}

/// Initializes the `log` crate bridge to forward all records to the given logger.
pub fn init(logger: Logger) -> Result<(), log::SetLoggerError> {
    let level = match logger.config().level {
        Level::Trace => log::LevelFilter::Trace,
        Level::Debug => log::LevelFilter::Debug,
        Level::Info | Level::Success => log::LevelFilter::Info,
        Level::Warn => log::LevelFilter::Warn,
        Level::Error | Level::Fatal => log::LevelFilter::Error,
    };
    log::set_boxed_logger(Box::new(LogBridge { logger }))?;
    log::set_max_level(level);
    Ok(())
}
