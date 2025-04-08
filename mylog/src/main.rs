mod mylog;

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
        Box::new(mylog::ConsoleLogger::new(
            mylog::Console::Stdout,
            log::Level::Trace,
            mylog::default_format,
        )),
        Box::new(mylog::ConsoleLogger::new(
            mylog::Console::Stderr,
            log::Level::Warn,
            mylog::default_format,
        )),
    ];
    mylog::RootLogger::init(loggers, log::Level::Trace);
    log::trace!("test");
    moda::modb::test();
}
