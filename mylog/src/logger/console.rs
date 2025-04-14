use chrono::Local;
use log::{Level, Log, Metadata, Record};

use super::{FormatArgs, translate_args};

#[derive(Debug, Clone, Copy)]
pub enum Console {
    Stdout,
    Stderr,
}

pub struct ConsoleLogger {
    level: Level,
    console: Console,
    color: bool,
    formatter: Box<dyn Fn(FormatArgs) -> String + Send + Sync>,
}

impl ConsoleLogger {
    pub fn new_boxed<F>(console: Console, level: Level, formatter: F) -> Box<dyn Log>
    where
        F: Fn(FormatArgs) -> String + Send + Sync + 'static,
    {
        let color = is_console(console);
        Box::new(Self {
            level,
            console,
            color,
            formatter: Box::new(formatter),
        })
    }
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
        let level_str = if self.color {
            match level {
                Level::Error => format!("{COL_RED}[{level:5}]{COL_RESET}"),
                Level::Warn => format!("{COL_YELLOW}[{level:5}]{COL_RESET}"),
                Level::Info => format!("{COL_GREEN}[{level:5}]{COL_RESET}"),
                Level::Debug => format!("{COL_PURPLE}[{level:5}]{COL_RESET}"),
                _ => format!("[{level:5}]"),
            }
        } else {
            format!("[{level:5}]")
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

/// Stdout/Stderr is redirected?
///
/// If not, returns true. (colored output will be enabled)
fn is_console(console: Console) -> bool {
    let fd = match console {
        Console::Stdout => libc::STDOUT_FILENO,
        Console::Stderr => libc::STDERR_FILENO,
    };
    let ret = unsafe { libc::isatty(fd) };

    ret != 0
}
