#include "logger.h"
#include "config.h"
#include "util.h"
#include "exec.h"
#include "taskserver.h"
#include "system/system.h"
#include "task/task.h"
#include "web/webpage.h"
#include <unistd.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <getopt.h>
#include <signal.h>
#include <cstdlib>
#include <string>
#include <thread>
#include <json11.hpp>

namespace {

using namespace shanghai;
using namespace std::string_literals;

const char * const PidFileName = "shanghai.pid";

class SafeFd {
public:
	explicit SafeFd(int fd) : m_fd(fd) {}
	~SafeFd() { close(m_fd); m_fd = -1; }
	int Get() { return m_fd; }

private:
	int m_fd;
};

// 処理したいシグナルを全てブロックし、シグナル処理スレッドでハンドルする
// ブロックしたシグナルセットを sigset に返す
void SetupSignalMask(sigset_t &sigset)
{
	int ret;

	util::SysCall(sigemptyset(&sigset));
	util::SysCall(sigaddset(&sigset, SIGINT));
	util::SysCall(sigaddset(&sigset, SIGTERM));
	util::SysCall(sigaddset(&sigset, SIGHUP));
	util::SysCall(sigaddset(&sigset, SIGUSR1));
	ret = pthread_sigmask(SIG_BLOCK, &sigset, NULL);
	if (ret != 0) {
		throw std::system_error(ret, std::generic_category());
	}
}

// シグナル処理スレッド (SIGUSR1 で終了)
void SignalThreadEntry(const sigset_t &sigset,
	std::unique_ptr<TaskServer> &server)
{
	int ret, sig;

	while (1) {
		// このスレッドでブロックを解除し sigset のうちどれかが届くまで待つ
		// 戻るときに再度ブロック
		ret = sigwait(&sigset, &sig);
		if (ret != 0) {
			throw std::system_error(ret, std::generic_category());
		}
		logger.Log(LogLevel::Info, "Signal: %d", sig);

		switch (sig) {
		case SIGINT:
			logger.Log(LogLevel::Info, "SIGINT");
			server->RequestShutdown(ServerResult::Shutdown);
			break;
		case SIGTERM:
			logger.Log(LogLevel::Info, "SIGTERM");
			server->RequestShutdown(ServerResult::Shutdown);
			break;
		case SIGHUP:
			logger.Log(LogLevel::Info, "SIGHUP");
			server->RequestShutdown(ServerResult::Reboot);
			break;
		case SIGUSR1:
			logger.Log(LogLevel::Info, "SIGUSR1");
			goto EXIT;
		default:
			logger.Log(LogLevel::Fatal, "Unknown signal: %d", sig);
			throw std::logic_error("unknown signal");
		}
	}
EXIT:
	logger.Log(LogLevel::Info, "Signal thread exit");
}

struct BootOpts {
	bool daemon = false;
};
BootOpts boot_opts;

void ParseArgs(int argc, char * const argv[])
{
	static const struct option long_opts[] = {
		{ "help",	no_argument,	nullptr,	'h' },
		{ "daemon",	no_argument,	nullptr,	'd' },
		{ 0, 0, 0, 0 },
	};
	const char *help_msg = R"(Usage:
--help
    Print this help and exit.
--daemon
    Start as daemon mode. (no stdin/stdout/stderr)
)";

	int c;
	int option_index = 0;
	while ((c = getopt_long(argc, argv, "", long_opts, &option_index)) != -1) {
		switch (c) {
		case 'h':
			std::puts(help_msg);
			std::exit(0);
		case 'd':
			boot_opts.daemon = true;
			break;
		case '?':
			// エラーは stderr に出してくれるのでそれに任せることにする
			std::exit(EXIT_FAILURE);
		default:
			throw std::logic_error("unknown getopt_long");
		}
	}
}

// 設定ファイル名
const std::array<const char *, 3> ConfigFiles = {
	"tw.json",
	"config.default.json",
	"config.json",
};

// タスクの登録
void SetupTasks(const std::unique_ptr<TaskServer> &server)
{
	server->RegisterPeriodicTask(
		std::make_unique<task::TaskConsumeTask>([](const struct tm &tm) {
			return true;
		}));
	server->RegisterPeriodicTask(
		std::make_unique<task::HealthCheckTask>([](const struct tm &tm) {
			const std::array<int, 2> hours = { 6, 18 };
			return tm.tm_min == 0 && std::find(
				hours.begin(), hours.end(), tm.tm_hour) != hours.end();
		}));
	server->RegisterPeriodicTask(
		std::make_unique<task::DdnsTask>([](const struct tm &tm) {
			return tm.tm_min == 0 && tm.tm_hour == 3;
		}));
	server->RegisterPeriodicTask(
		std::make_unique<task::TwitterTask>([](const struct tm &tm) {
			return (tm.tm_min + 2) % 5 == 0;
		}));
	server->RegisterPeriodicTask(
		std::make_unique<task::RandomTweetTask>([](const struct tm &tm) {
			return tm.tm_min == 0 && tm.tm_hour == 10;
		}));
}

void BootMsg(TaskServer &server, const std::atomic<bool> &cancel)
{
	auto &sys_info = system::Get().sys_info;
	auto &twitter = system::Get().twitter;

	std::string git_branch, git_hash;
	{
		Process p("/usr/bin/git",
			{"rev-parse", "--symbolic-full-name", "HEAD"});
		p.WaitForExit();
		git_branch = util::OneLine(p.GetOut());
	}
	{
		Process p("/usr/bin/git", {"rev-parse", "HEAD"});
		p.WaitForExit();
		git_hash = util::OneLine(p.GetOut());
	}

	sys_info.GetAndSet(
		[&git_branch, &git_hash]
		(system::SysInfoData &data) {
			data.start_time = std::time(nullptr);
			data.git_branch = git_branch;
			data.git_hash = git_hash;
		});

	std::string msg;
	msg += '[';
	msg += util::DateTimeStr();
	msg += "] Boot...\n";
	msg += git_branch;
	msg += '\n';
	msg += git_hash;

	twitter.Tweet(msg);
}

}	// namespace

#ifndef DISABLE_MAIN
int main(int argc, char *argv[])
{
	// コマンドライン引数のパース
	ParseArgs(argc, argv);

	if (boot_opts.daemon) {
		// `cd /` ?
		int nochdir = 1;
		// stdin/out/err を閉じて /dev/null にリダイレクトするか
		int noclose = 0;
		// fork() して親プロセスは _exit(0)
		int ret = daemon(nochdir, noclose);
		// fork() に失敗した場合は親プロセスに返る
		// 成功した場合は親プロセスは _exit(0) する
		// その後の失敗は子プロセスに失敗が返る
		if (ret < 0) {
			// 子プロセスで stderr が閉じられた後にエラーを返した場合
			// エラー詳細が虚空に消えるがその場合は諦める
			// (ファイルをいじりだすより前に子プロセス一本モードにしておきたいため)
			// 少なくとも glibc の実装では起こらなさそう
			perror("daemon()");
			return EXIT_FAILURE;
		}
	}

	// ログシステムの設定
	if (!boot_opts.daemon) {
		logger.AddStdOut(LogLevel::Trace);
	}
	logger.AddFile(LogLevel::Info);
	logger.Log(LogLevel::Info, "Initializing (daemon=%s)",
		boot_opts.daemon ? "yes" : "no");

	try {
		{
			// pid ファイルを作る(新規作成できなければ失敗)
			int pid_fd_raw = util::SysCall(
				open(PidFileName, O_WRONLY | O_CREAT | O_EXCL, 0600));
			SafeFd pid_fd(pid_fd_raw);

			// 終了時に pid ファイルを消すよう登録する
			// std::quick_exit() では呼ばれない
			// fork した子プロセスが exec 失敗して終了する時など
			std::atexit([](){
				unlink(PidFileName);
			});

			// pid を文字列で書いて閉じる
			std::string pid_str = std::to_string(getpid());
			pid_str += '\n';
			util::SysCall(write(pid_fd.Get(), pid_str.c_str(), pid_str.size()));
			// close
		}

		// メインスレッドでシグナルマスクを設定する
		// メインスレッドから生成されたサブスレッドはこの設定を継承する
		// ref. man pthread_sigmask
		sigset_t sigset;
		SetupSignalMask(sigset);

		while (1) {
			// 設定ファイルのロード
			config.Clear();
			for (const auto &file_name : ConfigFiles) {
				logger.Log(LogLevel::Info, "Load: %s", file_name);
				config.LoadFile(file_name);
			}

			// システムコンポーネントの全初期化
			system::SafeSystem system;
			// web ページの初期化
			web::SetupPages();

			// サーバの作成、初期化
			auto server = std::make_unique<TaskServer>();
			SetupTasks(server);
			if (config.GetBool({"System", "AllTasksFirst"})) {
				logger.Log(LogLevel::Info, "Release all tasks for test");
				server->ReleaseAllForTest();
			}
			// 起動メッセージタスクを追加
			server->RegisterOneShotTask(std::make_unique<OneShotTask>(
				"BootMsg", BootMsg));

			// シグナル処理スレッドを立ち上げる
			// シグナルセットとタスクサーバへの参照を渡す
			std::thread sigth(SignalThreadEntry, sigset, std::ref(server));
			auto teardown = [&sigth]() {
				// シグナル処理スレッドを SIGUSR1 で終了させて join
				util::SysCall(kill(getpid(), SIGUSR1));
				sigth.join();
			};
			ServerResult result = ServerResult::None;
			try {
				// サーバスタート
				result = server->Run();
			}
			catch (...) {
				teardown();
				throw;
			}
			teardown();

			switch (result) {
			case ServerResult::Reboot:
			case ServerResult::ErrorReboot:
				break;
			case ServerResult::Shutdown:
				goto EXIT;
			case ServerResult::FatalShutdown:
				std::terminate();
				break;
			default:
				std::terminate();
				break;
			}
			// destruct server
			// destruct system
		}
	}
	catch (std::runtime_error &e) {
		logger.Log(LogLevel::Fatal, "Runtime error");
		logger.Log(LogLevel::Fatal, "%s", e.what());
		return EXIT_FAILURE;
	}
	catch (std::exception &e) {
		logger.Log(LogLevel::Fatal, "Fatal error");
		logger.Log(LogLevel::Fatal, "%s", e.what());
		throw;
	}
	catch (...) {
		throw;
	}
EXIT:
	return 0;
}
#endif
