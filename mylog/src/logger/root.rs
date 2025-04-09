use log::{Log, Metadata};

pub(super) struct RootLogger {
    inner: Vec<Box<dyn Log>>,
}

impl RootLogger {
    pub(super) fn new(loggers: Vec<Box<dyn Log>>) -> Self {
        Self { inner: loggers }
    }
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
