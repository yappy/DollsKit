use mylog::logger::{self, FileLogger, RotateOptions, RotateSize, default_formatter};

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
            file_count: 3,
            size: RotateSize::Enabled(1024),
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

    for i in 0..1024 {
        info!("This is a log rotate test {i}");
    }
}
