//! Rust 版管理人形。
//!
//! 設定ファイルの説明は [sys::config::Config] にある。

// ドキュメントはライブラリの外部仕様の説明のためではなく、
// private も含めた実装の解説のために生成する。
#![allow(rustdoc::private_intra_doc_links)]

use anyhow::Result;
use customlog::{ConsoleLogger, FileLogger, FlushGuard, RotateOptions, RotateSize};
use getopts::Options;
use log::{LevelFilter, error, info};
use std::env;
use sys::sysmod::SystemModules;
use sys::taskserver::{Control, RunResult, TaskServer};

/// ログのファイル出力先。
const FILE_LOG: &str = "shanghai.log";

const LOG_FILTER: &[&str] = &[module_path!(), "sys"];
const LOG_ROTATE_SIZE: usize = 1024 * 1024;
const LOG_ROTATE_COUNT: u16 = 10;
const LOG_BUF_SIZE: usize = 64 * 1024;

fn log_target_filter(target: &str) -> bool {
    LOG_FILTER.iter().any(|filter| target.starts_with(filter))
}

/// ロギングシステムを有効化する。
///
/// 出力先は stdout と ファイル。
/// ログレベルは Error, Warn, Info, Debug, Trace の5段階である。
/// フィルタは Info 以上、
/// ただし verbose モードの場合は stdout へは Trace 以上のログが出力される。
/// (debug build では自動的に)
///
/// * `opts` - 起動オプション。
fn init_log(verbose: bool) -> Result<FlushGuard> {
    // filter = Off, Error, Warn, Info, Debug, Trace
    let rotate_opts = RotateOptions {
        size: RotateSize::Enabled(LOG_ROTATE_SIZE),
        file_count: LOG_ROTATE_COUNT,
        ..Default::default()
    };

    let log_dir = utils::dir::cache_dir()?;
    let file_path = log_dir.join(FILE_LOG);
    let file_log = FileLogger::new_boxed(
        LevelFilter::Info,
        log_target_filter,
        customlog::default_formatter,
        &file_path,
        LOG_BUF_SIZE,
        rotate_opts,
    )?;
    let file_path = file_path.canonicalize()?;

    // -v または debug build なら最大出力にする
    let console_filter = if cfg!(debug_assertions) || verbose {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };
    let console_log = ConsoleLogger::new_boxed(
        customlog::Console::Stdout,
        console_filter,
        log_target_filter,
        customlog::default_formatter,
    );
    let loggers = vec![console_log, file_log];

    let guard = customlog::init(loggers, LevelFilter::Trace);
    info!("init log: {}", file_path.to_string_lossy());

    Ok(guard)
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
            error!("{why:#?}");
        }
    }
    {
        let mut discord = ctrl.sysmods().discord.lock().await;
        if let Err(why) = discord.say(&msg).await {
            error!("error on discord notification");
            error!("{why:#?}");
        }
    }

    Ok(())
}

/// システムメイン処理。
/// コマンドラインとデーモン化、ログの初期化の後に入る。
///
/// 設定データをロードする。
/// その後、システムモジュールとタスクサーバを初期化し、システムの実行を開始する。
///
/// * SIGUSR1: ログのフラッシュ
/// * SIGUSR2: なし
fn system_main() -> Result<()> {
    let config_dir = utils::dir::config_dir()?;

    let sigusr1 = || {
        info!("Flush log");
        log::logger().flush();
        None
    };
    let sigusr2 = || None;

    loop {
        info!("system main");
        info!("{}", verinfo::version_info());
        log::logger().flush();

        sys::config::load(&config_dir)?;

        let sysmods = SystemModules::new()?;
        let ts = TaskServer::new(sysmods);

        ts.spawn_oneshot_task("boot_msg", boot_msg_task);
        let run_result = ts.run(sigusr1, sigusr2);

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
pub fn main() -> Result<()> {
    //create_run_script()?;

    // コマンドライン引数のパース
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help");
    opts.optflag("v", "verbose", "Print verbose logs on stdout");
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

    let verbose = matches.opt_present("v");

    let _flush = init_log(verbose)?;

    system_main().map_err(|e| {
        error!("Error in system_main");
        error!("{e:#}");
        e
    })

    // drop(_flush)
}
