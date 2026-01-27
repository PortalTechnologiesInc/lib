use log::{Level, Metadata, Record};
use std::sync::Arc;

#[uniffi::export(with_foreign)]
pub trait LogCallback: Send + Sync {
    fn log(&self, entry: LogEntry);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
pub enum LogLevel {
    Error = 1,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => LogLevel::Error,
            Level::Warn => LogLevel::Warn,
            Level::Info => LogLevel::Info,
            Level::Debug => LogLevel::Debug,
            Level::Trace => LogLevel::Trace,
        }
    }
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::Error,
            LogLevel::Warn => Level::Warn,
            LogLevel::Info => Level::Info,
            LogLevel::Debug => Level::Debug,
            LogLevel::Trace => Level::Trace,
        }
    }
}
/// Represents a log entry with all relevant information
#[derive(Debug, Clone, uniffi::Record)]
pub struct LogEntry {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// Custom logger that calls a callback function for each log entry
pub struct CallbackLogger {
    callback: Arc<dyn LogCallback>,
    max_level: Level,
}

impl CallbackLogger {
    /// Create a new CallbackLogger with the specified callback function
    pub fn new(callback: Arc<dyn LogCallback>) -> Self {
        Self {
            callback,
            max_level: Level::Trace, // Default to logging everything
        }
    }

    /// Create a new CallbackLogger with a specific maximum log level
    pub fn with_max_level(callback: Arc<dyn LogCallback>, max_level: Level) -> Self {
        Self {
            callback,
            max_level,
        }
    }

    /// Initialize this logger as the global logger
    pub fn init(self) -> Result<(), log::SetLoggerError> {
        let max_level = self.max_level;
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(max_level.to_level_filter());
        Ok(())
    }
}

impl log::Log for CallbackLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let entry = LogEntry {
                level: record.level().into(),
                target: record.target().to_string(),
                message: record.args().to_string(),
                module_path: record.module_path().map(|s| s.to_string()),
                file: record.file().map(|s| s.to_string()),
                line: record.line(),
            };

            self.callback.log(entry);
        }
    }

    fn flush(&self) {
        // Nothing to flush for callback-based logging
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, warn};
    use std::sync::{Arc, Mutex};

    // NOTE: only one test can be run at a time because the logger is global

    #[test]
    #[ignore]
    fn test_callback_logger() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let logs_clone = logs.clone();

        struct Container(Arc<Mutex<Vec<LogEntry>>>);
        impl LogCallback for Container {
            fn log(&self, entry: LogEntry) {
                self.0.lock().unwrap().push(entry);
            }
        }

        let callback = Arc::new(Container(logs_clone));

        let logger = CallbackLogger::new(callback);
        logger.init().unwrap();

        info!("Test info message");
        warn!("Test warning message");
        error!("Test error message");
        debug!("Test debug message");

        let captured_logs = logs.lock().unwrap();
        assert_eq!(captured_logs.len(), 4);
        assert_eq!(captured_logs[0].level, LogLevel::Info);
        assert_eq!(captured_logs[0].message, "Test info message");
        assert_eq!(captured_logs[1].level, LogLevel::Warn);
        assert_eq!(captured_logs[1].message, "Test warning message");
    }

    #[test]
    #[ignore]
    fn test_callback_logger_with_max_level() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let logs_clone = logs.clone();

        struct Container(Arc<Mutex<Vec<LogEntry>>>);
        impl LogCallback for Container {
            fn log(&self, entry: LogEntry) {
                self.0.lock().unwrap().push(entry);
            }
        }

        let callback = Arc::new(Container(logs_clone));

        let logger = CallbackLogger::with_max_level(callback, Level::Warn);
        logger.init().unwrap();

        info!("This should not be logged");
        warn!("This should be logged");
        error!("This should also be logged");

        let captured_logs = logs.lock().unwrap();
        assert_eq!(captured_logs.len(), 2);
        assert_eq!(captured_logs[0].level, LogLevel::Warn);
        assert_eq!(captured_logs[1].level, LogLevel::Error);
    }
}
