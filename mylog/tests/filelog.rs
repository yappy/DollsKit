use mylog::logger::{self, FileLogger, RotateOptions, default_formatter};

use log::{Level, debug, error, info, trace, warn};

#[test]
fn integration_test() {
    const BUF_SIZE: usize = 64;

    let logger = FileLogger::new(
        Level::Trace,
        default_formatter,
        "testlog.log",
        BUF_SIZE,
        RotateOptions {
            ..Default::default()
        },
    )
    .unwrap();
    let _flush = logger::init(vec![logger], Level::Trace);

    trace!("This is a test.");
    debug!("This is a test.");
    info!("This is a test.");
    warn!("This is a test.");
    error!("This is a test.");
}
