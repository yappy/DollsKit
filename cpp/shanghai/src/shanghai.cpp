#include "logger.h"
#include "config.h"
#include "taskserver.h"
#include "task/task.h"
#include <unistd.h>
#include <signal.h>
#include <cstdio>
#include <string>
#include <thread>
#include <json11.hpp>

namespace {

using namespace shanghai;
using namespace std::string_literals;

// 設定ファイル名
const char * const ConfigFile = "config.json";
const char * const ConfigFileFallback = "config.template.json";

void SetupTasks(const std::unique_ptr<TaskServer> &server)
{
	server->RegisterPeriodicTask(
		std::make_unique<task::DdnsTask>([](const struct tm &) {
			return true;
		}));
}

// 負の返り値の場合に errno から system_error を生成して投げる
inline void CheckedSysCall(int ret)
{
	if (ret < 0) {
		throw std::system_error(errno, std::generic_category());
	}
}

void SetupSignalMask(sigset_t &sigset)
{
	int ret;

	CheckedSysCall(sigemptyset(&sigset));
	CheckedSysCall(sigaddset(&sigset, SIGINT));
	CheckedSysCall(sigaddset(&sigset, SIGHUP));
	CheckedSysCall(sigaddset(&sigset, SIGUSR1));
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

}	// namespace

#ifndef DISABLE_MAIN
int main()
{
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

			// サーバの作成、初期化
			auto server = std::make_unique<TaskServer>();
			SetupTasks(server);

			// シグナル処理スレッドを立ち上げる
			// シグナルセットとタスクサーバへの参照を渡す
			std::thread sigth(SignalThreadEntry, sigset, std::ref(server));

			// Debug build only
#ifndef NDEBUG
			logger.Log(LogLevel::Warn, "Release all tasks for test");
			server->ReleaseAllForTest();
#endif
			// サーバスタート
			ServerResult result = server->Run();

			// シグナル処理スレッドを SIGUSR1 で終了させて join
			CheckedSysCall(kill(getpid(), SIGUSR1));
			sigth.join();

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
