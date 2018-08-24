#include "logger.h"
#include "config.h"
#include "taskserver.h"
#include <cstdio>
#include <string>
#include <json11.hpp>

namespace {

using namespace shanghai;
using namespace std::string_literals;

// TODO: 今はテンプレートをそのまま読む
const char * const ConfigFileName = "config.template.json";

class TestTask : public PeriodicTask {
public:
	std::string GetName() override
	{
		return "TestTask"s;
	}
	bool CheckRelease(const struct tm &local_time) override
	{
		return true;
	}
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override
	{
		logger.Log(LogLevel::Info, "test task");
	}
};

}	// namespace

#ifndef DISABLE_MAIN
int main()
{
	// ログシステムの設定
	logger.AddStdOut(LogLevel::Trace);
	logger.AddFile(LogLevel::Info);

	try {
		// 設定ファイルのロード
		logger.Log(LogLevel::Info, "Load config file");
		config.Load(ConfigFileName);

		while (1) {
			auto server = std::make_unique<TaskServer>();

			server->RegisterPeriodicTask(std::make_unique<TestTask>());
			ServerResult result = server->Run();

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
