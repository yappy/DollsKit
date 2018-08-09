#include <cstdio>
#include "logger.h"

int main()
{
	using namespace shanghai;

	std::puts("hello, shanghai");

	Logger logger;
	logger.AddStdOut(LogLevel::Trace);
	logger.Log(LogLevel::Info, "test log");

	return 0;
}
