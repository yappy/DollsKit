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

	auto server = std::make_unique<TaskServer>();
	server->Run();

	return 0;
}
#endif
