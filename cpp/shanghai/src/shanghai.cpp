#include <cstdio>
#include "logger.h"

int main()
{
	using namespace shanghai;

	std::puts("hello, shanghai");

	Logger logger;
	logger.AddStdOut(LogLevel::Trace);
	logger.AddFile(LogLevel::Trace);
	for (int i = 0; i < 100 * 1000; i++) {
		logger.Log(LogLevel::Info, "test log %8d %256s", i, "");
	}

	return 0;
}
