use chrono::Local;
use log::{Level, Log, Metadata, Record};

use super::{FormatArgs, translate_args};

pub enum Console {
    Stdout,
    Stderr,
}

pub struct ConsoleLogger {
    level: Level,
    console: Console,
    formatter: Box<dyn Fn(FormatArgs) -> String + Send + Sync>,
}

impl Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        const COL_RED: &str = "\x1b[31m";
        const COL_YELLOW: &str = "\x1b[33m";
        const COL_GREEN: &str = "\x1b[32m";
        const COL_PURPLE: &str = "\x1b[35m";
        const COL_RESET: &str = "\x1b[0m";

        let timestamp = Local::now();

        let level = record.level();
        let level_str = match level {
            Level::Error => format!("{COL_RED}[{level:5}]{COL_RESET}"),
            Level::Warn => format!("{COL_YELLOW}[{level:5}]{COL_RESET}"),
            Level::Info => format!("{COL_GREEN}[{level:5}]{COL_RESET}"),
            Level::Debug => format!("{COL_PURPLE}[{level:5}]{COL_RESET}"),
            _ => format!("[{level:5}]"),
        };
        let mut args = translate_args(record, timestamp);
        args.level_str = level_str;

        let output = self.formatter.as_ref()(args);
        match self.console {
            Console::Stdout => {
                println!("{output}");
            }
            Console::Stderr => {
                eprintln!("{output}");
            }
        }
    }

    fn flush(&self) {}
}

impl ConsoleLogger {
    pub fn new<F>(console: Console, level: Level, formatter: F) -> Self
    where
        F: Fn(FormatArgs) -> String + Send + Sync + 'static,
    {
        Self {
            level,
            console,
            formatter: Box::new(formatter),
        }
    }
}
