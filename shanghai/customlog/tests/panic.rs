use customlog::logger::{self, FileLogger, RotateOptions, default_formatter};
use log::{Level, debug, error, info, trace, warn};

#[test]
#[ignore]
#[should_panic]
fn log_panic() {
    let logger = FileLogger::new_boxed(
        Level::Trace,
        default_formatter,
        "panic.log",
        64,
        RotateOptions {
            ..Default::default()
        },
    )
    .unwrap();
    let _flush = logger::init(vec![logger], Level::Trace);

    trace!("This is a panic test.");
    trace!("This is a test.");
    debug!("This is a test.");
    info!("This is a test.");
    warn!("This is a test.");
    error!("This is a test.");
    panic!("Test Panic");
}
