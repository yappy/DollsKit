use chrono::{DateTime, Local, SecondsFormat};
use log::{Level, Log, Metadata, Record, SetLoggerError};

pub struct FormatArgs<'a> {
    pub timestamp: DateTime<Local>,
    pub level: Level,
    pub level_str: String,
    pub target: &'a str,
    pub body: String,
    pub module: &'a str,
    pub file: &'a str,
    pub line: String,
}

pub fn default_format(args: FormatArgs) -> String {
    match args.level {
        Level::Trace => format!(
            "{} {} {} {} ({} {}:{})",
            args.timestamp.to_rfc3339_opts(SecondsFormat::Secs, false),
            args.level_str,
            args.target,
            args.body,
            args.module,
            args.file,
            args.line
        ),
        _ => format!(
            "{} {} {} {}",
            args.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true),
            args.level_str,
            args.target,
            args.body
        ),
    }
}

fn translate_args<'a>(record: &Record<'a>, timestamp: DateTime<Local>) -> FormatArgs<'a> {
    let level = record.level();
    let level_str = level.to_string();
    let target = record.target();
    let body = record.args().to_string();
    let module = record.module_path().unwrap_or("unknown");
    let file = record.file().unwrap_or("unknown");
    let line = record
        .line()
        .map_or("unknown".to_string(), |n| n.to_string());

    FormatArgs {
        timestamp,
        level,
        level_str,
        target,
        body,
        module,
        file,
        line,
    }
}

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
