mod console;
mod file;
mod root;

// Raname and export
pub use console::{Console, ConsoleLogger};

use chrono::{DateTime, Local, SecondsFormat};
use log::{Level, Log, Record, SetLoggerError};
use root::RootLogger;

/// [FlushGuard] must be dropped on the end of the program.
///  Panic if a logger is already set.
pub fn init(loggers: Vec<Box<dyn Log>>, level: Level) -> FlushGuard {
    init_raw(loggers, level).unwrap()
}

/// Initialize with [ConsoleLogger] + [Console::Stdout] + [default_formatter].
///
/// [FlushGuard] must be dropped on the end of the program.
/// Ignore errors if a logger is already set.
///
/// This function is intended to be called on test start.
/// `cargo test -- --nocapture` option is needed to see the log.
#[allow(unused)]
pub fn init_for_test(level: Level) -> FlushGuard {
    let loggers = vec![ConsoleLogger::new(
        Console::Stdout,
        level,
        default_formatter,
    )];
    if let Ok(flush) = init_raw(loggers, level) {
        // OK
        flush_on_panic();
        flush
    } else {
        // already registered
        FlushGuard
    }
}

fn init_raw(loggers: Vec<Box<dyn Log>>, level: Level) -> Result<FlushGuard, SetLoggerError> {
    let log = RootLogger::new(loggers);
    log::set_max_level(level.to_level_filter());
    log::set_boxed_logger(Box::new(log))?;

    Ok(FlushGuard)
}

#[must_use]
/// On drop, calls `log::logger()::flush()`.
pub struct FlushGuard;

impl Drop for FlushGuard {
    fn drop(&mut self) {
        log::logger().flush();
        eprintln!("flush!")
    }
}

/// Set panic hook that calls [FlushGuard::drop].
fn flush_on_panic() {
    let old = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // execute old handler
        old(info);
        // flush on drop
        let _ = FlushGuard;
    }));
}

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

pub fn default_formatter(args: FormatArgs) -> String {
    match args.level {
        Level::Trace => format!(
            "{} {} {} ({} {}:{}) {}",
            args.timestamp.to_rfc3339_opts(SecondsFormat::Secs, false),
            args.level_str,
            args.target,
            args.module,
            args.file,
            args.line,
            args.body,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn log1() {
        let _ = init_for_test(Level::Trace);

        log::error!("This is a test {}", 42);
        log::warn!("This is a test {}", 42);
        log::info!("This is a test {}", 42);
        log::debug!("This is a test {}", 42);
        log::trace!("This is a test {}", 42);
    }
}
