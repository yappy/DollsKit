extern crate daemonize;

use std::fs::File;
use daemonize::Daemonize;

const STDERR_FILE: &str = "./stderr.txt";
const PID_FILE: &str = "./rshanghai.pid";

/// Create an stderr redirect file, and do daemonize.
/// If succeeded, the main process will exit successfully and
/// the forked child process continues to run as a daemon process.
/// If failed, the main process will exit(1).
fn daemon() {
    let stderr = File::create(STDERR_FILE);
    if let Err(e) = stderr {
        eprintln!("Open {} error: {}", STDERR_FILE, e);
        std::process::exit(1);
    }
    let stderr = stderr.unwrap();

    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        //.chown_pid_file(true)
        .working_directory(".")
        //.user("nobody")
        //.group("daemon")
        //.stdout(stdout)
        .stderr(stderr);

    if let Err(e) = daemonize.start() {
        eprintln!("Daemonize error: {}", e);
        std::process::exit(1);
    }
}

fn main() {
    daemon();
}
