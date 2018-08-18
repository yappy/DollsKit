#include "logger.h"
#include "taskserver.h"
#include <cstdio>
#include <string>
#include <json11.hpp>

namespace {

using namespace shanghai;
using namespace std::string_literals;

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
	std::puts("hello, shanghai");

	logger.AddStdOut(LogLevel::Trace);
	logger.AddFile(LogLevel::Trace);

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
EXIT:
	return 0;
}
#endif
