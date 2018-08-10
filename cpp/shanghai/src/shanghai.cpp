#include <cstdio>
#include "logger.h"
#include "taskserver.h"
#include <json11.hpp>

int main()
{
	using namespace shanghai;

	std::puts("hello, shanghai");

	logger = std::make_unique<Logger>();
	logger->AddStdOut(LogLevel::Trace);
	logger->AddFile(LogLevel::Trace);

	auto server = std::make_unique<TaskServer>();

	const char *sample = "{\"a\": 3.14}";
	std::string err;
	auto json = json11::Json::parse(sample, err);
	logger->Log(LogLevel::Info, "%s", json.dump().c_str());
	logger->Log(LogLevel::Error, "%s", err.c_str());

	return 0;
}
