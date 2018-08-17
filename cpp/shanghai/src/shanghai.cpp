#include <cstdio>
#include "logger.h"
#include "taskserver.h"
#include <json11.hpp>

#ifndef DISABLE_MAIN
int main()
{
	using namespace shanghai;

	std::puts("hello, shanghai");

	logger.AddStdOut(LogLevel::Trace);
	logger.AddFile(LogLevel::Trace);

	while (1) {
		auto server = std::make_unique<TaskServer>();
		// テスト用に外部から3秒後にシャットダウンを要求する
		std::thread th([&server]() {
			std::this_thread::sleep_for(std::chrono::seconds(3));
			server->RequestShutdown(ServerResult::Shutdown);
		});
		th.detach();
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
