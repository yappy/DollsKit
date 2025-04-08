use log::{Level, Log, Metadata, SetLoggerError};

pub struct RootLogger {
    inner: Vec<Box<dyn Log>>,
}

impl Log for RootLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.inner.iter().any(|logger| logger.enabled(metadata))
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        for logger in &self.inner {
            logger.log(record);
        }
    }

    fn flush(&self) {
        for logger in &self.inner {
            logger.flush();
        }
    }
}

impl RootLogger {
    /// Panic if a logger is already set.
    pub fn init(loggers: Vec<Box<dyn Log>>, level: Level) {
        Self::init_raw(loggers, level).unwrap();
    }

    /// Ignore errors if a logger is already set.
    #[allow(unused)]
    pub fn init_for_test(loggers: Vec<Box<dyn Log>>, level: Level) {
        let _ = Self::init_raw(loggers, level);
    }

    fn init_raw(loggers: Vec<Box<dyn Log>>, level: Level) -> Result<(), SetLoggerError> {
        let log = RootLogger { inner: loggers };
        log::set_max_level(level.to_level_filter());

        log::set_boxed_logger(Box::new(log))
    }
}
