#include "logger.h"
#include "config.h"
#include "util.h"
#include "taskserver.h"
#include "system/system.h"
#include "task/task.h"
#include "web/webpage.h"
#include <unistd.h>
#include <getopt.h>
#include <signal.h>
#include <cstdio>
#include <string>
#include <thread>
#include <json11.hpp>

namespace {

using namespace shanghai;
using namespace std::string_literals;

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
const char * const ConfigFile = "config.json";
const char * const ConfigFileFallback = "config.template.json";

// タスクの登録
void SetupTasks(const std::unique_ptr<TaskServer> &server)
{
	server->RegisterPeriodicTask(
		std::make_unique<task::HealthCheckTask>([](const struct tm &) {
			return true;
		}));
	server->RegisterPeriodicTask(
		std::make_unique<task::DdnsTask>([](const struct tm &) {
			return true;
		}));
	server->RegisterPeriodicTask(
		std::make_unique<task::TwitterTask>([](const struct tm &) {
			return true;
		}));
}

}	// namespace

#ifndef DISABLE_MAIN
int main(int argc, char *argv[])
{
	// コマンドライン引数のパース
	ParseArgs(argc, argv);

	// ログシステムの設定
	logger.AddStdOut(LogLevel::Trace);
	logger.AddFile(LogLevel::Info);

	// メインスレッドでシグナルマスクを設定する
	// メインスレッドから生成されたサブスレッドはこの設定を継承する
	// ref. man pthread_sigmask
	sigset_t sigset;
	SetupSignalMask(sigset);

	try {
		while (1) {
			// 設定ファイルのロード
			try {
				logger.Log(LogLevel::Info, "Load: %s", ConfigFile);
				config.LoadFile(ConfigFile);
			}
			catch (std::runtime_error &e) {
#ifndef NDEBUG
				// DEBUG
				logger.Log(LogLevel::Error, "Load config failed");
				logger.Log(LogLevel::Error, "%s", e.what());
				logger.Log(LogLevel::Warn,
					"DEBUG BUILD ONLY: load template file instead");
				logger.Log(LogLevel::Info, "Load: %s", ConfigFileFallback);
				config.LoadFile(ConfigFileFallback);
#else
				// RELEASE
				throw;
#endif
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
			catch(...) {
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
		return 1;
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
