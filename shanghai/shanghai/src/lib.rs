//! Rust 版管理人形。
//!
//! 設定ファイルの説明は [sys::config::Config] にある。

// ドキュメントはライブラリの外部仕様の説明のためではなく、
// private も含めた実装の解説のために生成する。
#![allow(rustdoc::private_intra_doc_links)]

pub mod sys;
mod sysmod;

use anyhow::Result;
use daemonize::Daemonize;
use getopts::Options;
use log::{LevelFilter, error, info};
use simplelog::format_description;
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, SharedLogger, TermLogger, TerminalMode, WriteLogger,
};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::os::unix::prelude::*;
use sys::taskserver::{Control, RunResult, TaskServer};
use sysmod::SystemModules;

/// ログフィルタのためのクレート名。[module_path!] による。
const CRATE_NAME: &str = module_path!();

/// デーモン化の際に指定する stdout のリダイレクト先。
const FILE_STDOUT: &str = "stdout.txt";
/// デーモン化の際に指定する stderr のリダイレクト先。
const FILE_STDERR: &str = "stderr.txt";
/// Cron 用シェルスクリプトの出力先。
const FILE_EXEC_SH: &str = "exec.sh";
/// Cron 用シェルスクリプトの出力先。
const FILE_KILL_SH: &str = "kill.sh";
/// Cron 設定例の出力先。
const FILE_CRON: &str = "cron.txt";
/// デーモン化の際に指定する pid ファイルパス。
const FILE_PID: &str = "shanghai.pid";
/// ログのファイル出力先。
const FILE_LOG: &str = "shanghai.log";

/// stdout, stderr をリダイレクトし、デーモン化する。
///
/// ファイルオープンに失敗したら exit(1) する。
/// デーモン化に失敗したら exit(1) する。
/// 成功した場合は元の親プロセスは正常終了し、子プロセスが以降の処理を続行する。
fn daemon() {
    let stdout = match File::create(FILE_STDOUT) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Open {FILE_STDOUT} error: {e}");
            std::process::exit(1);
        }
    };
    let stderr = match File::create(FILE_STDERR) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Open {FILE_STDERR} error: {e}");
            std::process::exit(1);
        }
    };

    let daemonize = Daemonize::new()
        .pid_file(FILE_PID)
        //.chown_pid_file(true)
        .working_directory(".")
        //.user("nobody")
        //.group("daemon")
        .stdout(stdout)
        .stderr(stderr);

    if let Err(e) = daemonize.start() {
        eprintln!("Daemonize error: {e}");
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
        .add_filter_allow_str(CRATE_NAME)
        .set_time_offset_to_local()
        .unwrap()
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .build();
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(FILE_LOG)
        .unwrap();

    // filter = Off, Error, Warn, Info, Debug, Trace
    let loggers: Vec<Box<dyn SharedLogger>> = if is_daemon {
        vec![WriteLogger::new(LevelFilter::Info, config, file)]
    } else {
        vec![
            TermLogger::new(
                LevelFilter::Trace,
                config.clone(),
                TerminalMode::Stdout,
                ColorChoice::Never,
            ),
            WriteLogger::new(LevelFilter::Info, config, file),
        ]
    };
    CombinedLogger::init(loggers).unwrap();
}

/// 起動時に一度だけブートメッセージをツイートするタスク。
async fn boot_msg_task(ctrl: Control) -> Result<()> {
    let build_info = verinfo::version_info();
    // 同一テキストをツイートしようとするとエラーになるので日時を含める
    let now = chrono::Local::now();
    let now = now.format("%F %T %:z");
    let msg = format!("[{now}] Boot...\n{build_info}");

    {
        let mut twitter = ctrl.sysmods().twitter.lock().await;
        if let Err(why) = twitter.tweet(&msg).await {
            error!("error on tweet");
            error!("{:#?}", why);
        }
    }
    {
        let mut discord = ctrl.sysmods().discord.lock().await;
        if let Err(why) = discord.say(&msg).await {
            error!("error on discord notification");
            error!("{:#?}", why);
        }
    }

    Ok(())
}

/// システムメイン処理。
/// コマンドラインとデーモン化、ログの初期化の後に入る。
///
/// 設定データをロードする。
/// その後、システムモジュールとタスクサーバを初期化し、システムの実行を開始する。
fn system_main() -> Result<()> {
    loop {
        info!("system main");
        info!("{}", verinfo::version_info());

        sys::config::load()?;

        let sysmods = SystemModules::new()?;
        let ts = TaskServer::new(sysmods);

        ts.spawn_oneshot_task("boot_msg", boot_msg_task);
        let run_result = ts.run();

        info!("task server dropped");

        match run_result {
            RunResult::Shutdown => {
                info!("result: shutdown");
                break;
            }
            RunResult::Reboot => {
                info!("result: reboot");
            }
        }
    }

    Ok(())
}

/// 実行可能パーミッション 755 でファイルを作成して close せずに返す。
fn create_sh(path: &str) -> Result<File> {
    let f = File::create(path)?;

    let mut perm = f.metadata()?.permissions();
    perm.set_mode(0o755);
    f.set_permissions(perm)?;

    Ok(f)
}

/// 実行ファイル絶対パスから便利なスクリプトを生成する。
///
/// [FILE_EXEC_SH], [FILE_KILL_SH], [FILE_CRON].
fn create_run_script() -> Result<()> {
    let exe = env::current_exe()?.to_string_lossy().to_string();
    let cd = env::current_dir()?.to_string_lossy().to_string();

    {
        let f = create_sh(FILE_EXEC_SH)?;
        let mut w = BufWriter::new(f);

        writeln!(&mut w, "#!/bin/bash")?;
        writeln!(&mut w, "set -euxo pipefail")?;
        writeln!(&mut w)?;
        writeln!(&mut w, "cd '{cd}'")?;
        writeln!(&mut w, "'{exe}' --daemon")?;
    }
    {
        let f = create_sh(FILE_KILL_SH)?;
        let mut w = BufWriter::new(f);

        writeln!(&mut w, "#!/bin/bash")?;
        writeln!(&mut w, "set -euxo pipefail")?;
        writeln!(&mut w)?;
        writeln!(&mut w, "cd '{cd}'")?;
        writeln!(&mut w, "kill `cat {FILE_PID}`")?;
    }
    {
        let f = File::create(FILE_CRON)?;
        let mut w = BufWriter::new(f);

        write!(
            &mut w,
            "# How to use
# $ crontab < {FILE_CRON}
# How to verify
# $ crontab -l

# workaround: wait for 30 sec to wait for network
# It seems that DNS fails just after reboot

@reboot sleep 30; cd {cd}; ./{FILE_EXEC_SH}
"
        )?;
    }

    Ok(())
}

/// コマンドラインのヘルプを表示する。
///
/// * `program` - プログラム名 (argv\[0\])。
/// * `opts` - パーサオブジェクト。
fn print_help(program: &str, opts: Options) {
    let brief = format!("Usage: {program} [options]");
    print!("{}", opts.usage(&brief));
}

/// エントリポイント。
///
/// コマンドラインとデーモン化、ログの初期化処理をしたのち、[system_main] を呼ぶ。
pub fn run() -> Result<()> {
    create_run_script()?;

    // コマンドライン引数のパース
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");
    opts.optflag("d", "daemon", "Run as daemon");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(fail) => {
            eprintln!("{fail}");
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
    } else {
        init_log(false);
    }

    system_main().map_err(|e| {
        error!("Error in system_main");
        error!("{:#}", e);
        e
    })
}
