//! Rust 版管理人形

// TODO: 最終的には外すこと
#![allow(dead_code)]
#![allow(unused_variables)]

mod sys;
mod sysmod;

use std::env;
use std::fs::{remove_file, File, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::io::{Write, Read};
use getopts::Options;
use simplelog::{ConfigBuilder, CombinedLogger, SharedLogger, WriteLogger, TermLogger};
use simplelog::{TerminalMode, ColorChoice};
use simplelog::format_description;
use log::{error, warn, info, LevelFilter};
use daemonize::Daemonize;
use sys::taskserver::{TaskServer, Control};
use sysmod::SystemModules;


/// デーモン化の際に指定する stdout のリダイレクト先。
const STDOUT_FILE: &str = "stdout.txt";
/// デーモン化の際に指定する stderr のリダイレクト先。
const STDERR_FILE: &str = "stderr.txt";
/// デーモン化の際に指定する pid ファイルパス。
const PID_FILE: &str = "rshanghai.pid";
/// ログのファイル出力先。
const LOG_FILE: &str = "rshanghai.log";

/// デフォルトの設定データ(json source)。
/// [include_str!] でバイナリに含める。
const DEF_CONFIG_JSON: &str = include_str!("res/config_default.json");
const TW_CONTENTS_JSON: &str = include_str!("res/tw_contents.json");
const CONFIG_FILE: &str = "config.json";
const CONFIG_DEF_FILE: &str = "config_default.json";

/// stdout, stderr をリダイレクトし、デーモン化する。
///
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

/// ロギングシステムを有効化する。
///
/// デーモンならファイルへの書き出しのみ、
/// そうでないならファイルと stdout へ書き出す。
///
/// ログレベルは Error, Warn, Info, Debug, Trace の5段階である。
/// ファイルへは Info 以上、stdout へは Trace 以上のログが出力される。
///
/// * `is_daemon` - デーモンかどうか。
fn init_log(is_daemon: bool) {
    let config = ConfigBuilder::new()
        .set_time_format_custom(format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
        .build();
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(LOG_FILE)
        .unwrap();

    // filter = Off, Error, Warn, Info, Debug, Trace
    let loggers: Vec<Box<dyn SharedLogger>> = if is_daemon {
        vec![
            WriteLogger::new(LevelFilter::Info, config, file),
        ]
    }
    else {
        vec![
            TermLogger::new(LevelFilter::Trace, config.clone(), TerminalMode::Stdout, ColorChoice::Never),
            WriteLogger::new(LevelFilter::Info, config, file),
        ]
    };
    CombinedLogger::init(loggers).unwrap();
}

/// 設定データをロードする。
fn load_config() -> Result<(), ()> {
    {
        // デフォルト設定ファイルを削除する
        info!("Remove {}", CONFIG_DEF_FILE);
        if let Err(e) = remove_file(CONFIG_DEF_FILE) {
            warn!("Removing {} failed (the first time execution?): {}",
                CONFIG_DEF_FILE, e);
        }
        // デフォルト設定を書き出す
        // 600 でアトミックに必ず新規作成する、失敗したらエラー
        info!("Writing default config to {}", CONFIG_DEF_FILE);
        let f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(CONFIG_DEF_FILE);
        let mut f = match f {
            Ok(f) => f,
            Err(e) => {
                error!("Writing {} failed: {}", CONFIG_DEF_FILE, e);
                return Err(())
            },
        };
        f.write_all(DEF_CONFIG_JSON.as_bytes()).unwrap();
        info!("OK: written to {}", CONFIG_DEF_FILE);
        // close
    }

    let mut json_str = String::new();
    {
        // 設定ファイルを読む
        // open 後パーミッションを確認し、危険ならエラーとする
        info!("Loading config: {}", CONFIG_FILE);
        let f = OpenOptions::new()
            .read(true)
            .open(CONFIG_FILE);
        let mut f = match f {
            Ok(f) => f,
            Err(e) => {
                error!("Opening {} failed: {}", CONFIG_FILE, e);
                info!("HINT: Create {} and try again", CONFIG_FILE);
                return Err(())
            },
        };
        let metadata = f.metadata().expect("Cannot get metadata");
        let permissions = metadata.permissions();
        let masked = permissions.mode() & 0o777;
        if masked != 0o600 {
            error!("Config file permission is not 600: {:03o}", permissions.mode());
            return Err(());
        }
        if let Err(e) = f.read_to_string(&mut json_str) {
            error!("Read error: {}", e.to_string());
            return Err(());
        }
        info!("OK: {} loaded", CONFIG_FILE);
        // close
    }

    // json パースして設定システムを初期化
    let json_list = [DEF_CONFIG_JSON, TW_CONTENTS_JSON, &json_str];
    sys::config::init();
    for json_str in json_list {
        if let Err(msg) = sys::config::add_config(json_str)
        {
            error!("Config load failed: {}", msg);
            return Err(());
        }
    }

    Ok(())
}

async fn boot_tweet_task(ctrl: Control) -> Result<(), String> {
    // 同一テキストをツイートしようとするとエラーになるので日時を含める
    let build_info: &str = &*sys::version::VERSION_INFO;
    let msg = format!("[TODO: DATE] Boot...\n{}", build_info);

    {
        let mut twitter = ctrl.sysmods().twitter.write().await;
        twitter.tweet(&msg).await?;
    }

    Ok(())
}

/// システムメイン処理。
/// コマンドラインとデーモン化、ログの初期化の後に入る。
///
/// 設定データをロードする。
/// その後、システムモジュールとタスクサーバを初期化し、システムの実行を開始する。
fn system_main() {
    info!("{}", *sys::version::VERSION_INFO);

    load_config().expect("Load config failed");
    {
        let sysmods = SystemModules::new();
        let ts = TaskServer::new(sysmods);

        ts.sysmod_start();
        ts.spawn_oneshot_task("task1", boot_tweet_task);
        ts.run();
    }
    info!("task server dropped")
}

/// コマンドラインのヘルプを表示する。
///
/// * `program` - プログラム名 (argv\[0\])。
/// * `opts` - パーサオブジェクト。
fn print_help(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

/// エントリポイント。
///
/// コマンドラインとデーモン化、ログの初期化処理をしたのち、[system_main] を呼ぶ。
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
        print_help(program, opts);
        std::process::exit(0);
    }

    if matches.opt_present("d") {
        daemon();
        init_log(true);
    }
    else {
        init_log(false);
    }

    system_main();
}
