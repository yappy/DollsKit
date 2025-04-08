mod console;
mod root;

pub use root::RootLogger;
pub use console::{Console, ConsoleLogger};

use chrono::{DateTime, Local, SecondsFormat};
use log::{Level, Record};

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
