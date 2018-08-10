#include "logger.h"
#include <cstdarg>
#include <cstdio>
#include <ctime>

namespace shanghai {

namespace {

using mtx_guard = std::lock_guard<std::mutex>;

const uint32_t MsgLenMax = 512;
const uint32_t LogLenMax = 1024;

// 標準出力の実装
class StdOutTarget : public LogTarget {
public:
	StdOutTarget(LogLevel level) : LogTarget(level) {}
	virtual ~StdOutTarget() = default;

	virtual void Write(const char *str) override
	{
		std::puts(str);
	}
	virtual void Flush() override {}
};

}	// namespace

void Logger::Log(LogLevel level, const char *fmt, ...) noexcept
{
	std::time_t timestamp = std::time(nullptr);
	char msg[MsgLenMax] = "";
	char timestr[64] = "";
	char logstr[LogLenMax] = "";

	// msg <- sprintf(fmt, ...)
	{
		va_list arg;
		va_start(arg, fmt);
		std::vsnprintf(msg, sizeof(msg) - 1, fmt, arg);
		msg[sizeof(msg) - 1] = '\0';
		va_end(arg);
	}
	{
		mtx_guard lock(m_mtx);

		// ローカルタイムへの変換がスレッドセーフでないので一緒に排他する
		struct tm *local = std::localtime(&timestamp);
		if (std::strftime(timestr, sizeof(timestr), "%c", local) == 0) {
			timestr[0] = '\0';
		}

		std::snprintf(logstr, sizeof(logstr) - 1, "%s: %s", timestr, msg);
		logstr[sizeof(logstr) - 1] = '\0';

		for (auto &target : m_targets) {
			if (target->CheckLevel(level)) {
				try {
					target->Write(logstr);
				}
				catch (...) {
					std::fprintf(stderr, "Error on log write\n");
				}
			}
		}
	}	// unlock
}

void Logger::Flush() noexcept
{
	mtx_guard lock(m_mtx);
	for (auto &target : m_targets) {
		try {
			target->Flush();
		}
		catch (...) {
			std::fprintf(stderr, "Error on log flush\n");
		}
	}
	// unlock
}

void Logger::AddStdOut(LogLevel level)
{
	mtx_guard lock(m_mtx);
	auto target = std::make_unique<StdOutTarget>(level);
	m_targets.push_back(std::move(target));
	// unlock
}

}	// namespace shanghai
