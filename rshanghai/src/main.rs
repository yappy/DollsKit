extern crate daemonize;

use std::fs::File;
use daemonize::Daemonize;

const STDERR_FILE: &str = "./stderr.txt";
const PID_FILE: &str = "./rshanghai.pid";

/// stderr をリダイレクトし、デーモン化する。
/// stderr 用のファイルオープンに失敗したら exit(1) する。
/// デーモン化に失敗したら exit(1) する。
/// 成功した場合は元の親プロセスは正常終了し、子プロセスが以降の処理を続行する。
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
