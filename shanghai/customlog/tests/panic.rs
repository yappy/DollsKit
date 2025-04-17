use customlog::{FileLogger, RotateOptions, default_filter, default_formatter};
use log::{LevelFilter, debug, error, info, trace, warn};

#[test]
#[ignore]
#[should_panic]
fn log_panic() {
    let logger = FileLogger::new_boxed(
        LevelFilter::Trace,
        default_filter,
        default_formatter,
        "panic.log",
        64,
        RotateOptions {
            ..Default::default()
        },
    )
    .unwrap();
    let _flush = customlog::init(vec![logger], LevelFilter::Trace);

    trace!("This is a panic test.");
    trace!("This is a test.");
    debug!("This is a test.");
    info!("This is a test.");
    warn!("This is a test.");
    error!("This is a test.");
    panic!("Test Panic");
}
