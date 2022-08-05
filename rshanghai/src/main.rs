extern crate daemonize;
extern crate getopts;

use getopts::Options;
use std::env;
use std::fs::File;
use daemonize::Daemonize;

const STDOUT_FILE: &str = "./stdout.txt";
const STDERR_FILE: &str = "./stderr.txt";
const PID_FILE: &str = "./rshanghai.pid";

/// stdout, stderr をリダイレクトし、デーモン化する。
/// ファイルオープンに失敗したら exit(1) する。
/// デーモン化に失敗したら exit(1) する。
/// 成功した場合は元の親プロセスは正常終了し、子プロセスが以降の処理を続行する。
fn daemon() {
    let stdout = match File::create(STDOUT_FILE) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Open {} error: {}", STDOUT_FILE, e);
            std::process::exit(1);
        }
    };

    let stderr = match File::create(STDERR_FILE) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Open {} error: {}", STDERR_FILE, e);
            std::process::exit(1);
        }
    };

    let daemonize = Daemonize::new()
        .pid_file(PID_FILE)
        //.chown_pid_file(true)
        .working_directory(".")
        //.user("nobody")
        //.group("daemon")
        .stdout(stdout)
        .stderr(stderr);

    if let Err(e) = daemonize.start() {
        eprintln!("Daemonize error: {}", e);
        std::process::exit(1);
    }
}

fn print_help(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    // コマンドライン引数のパース
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");
    opts.optflag("d", "daemon", "Run as daemon");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(fail) => {
            eprintln!("{}", fail);
            std::process::exit(1);
        }
    };

    // --help がある場合は出力して終了
    if matches.opt_present("h") {
        print_help(&program, opts);
        std::process::exit(0);
    }

    if matches.opt_present("d") {
        daemon();
        println!("Daemon mode");
    }
    else {
        println!("Not daemon mode");
    }
}
