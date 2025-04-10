use mylog::logger;

mod moda {
    pub mod modb {
        pub fn test() {
            log::error!("This is a test {}", 42);
            log::warn!("This is a test {}", 42);
            log::info!("This is a test {}", 42);
            log::debug!("This is a test {}", 42);
            log::trace!("This is a test {}", 42);
        }
    }
}
fn main() {
    let loggers: Vec<Box<dyn log::Log>> = vec![
        Box::new(logger::ConsoleLogger::new(
            logger::Console::Stdout,
            log::Level::Trace,
            logger::default_formatter,
        )),
        Box::new(logger::ConsoleLogger::new(
            logger::Console::Stderr,
            log::Level::Warn,
            logger::default_formatter,
        )),
    ];
    let flush = logger::init(loggers, log::Level::Trace);

    log::trace!("test");
    moda::modb::test();

    drop(flush);
}
