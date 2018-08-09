#include "logger.h"
#include <cstdarg>
#include <cstdio>

namespace shanghai {

namespace {

using mtx_guard = std::lock_guard<std::mutex>;

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
	mtx_guard lock(m_mtx);
	for (auto &target : m_targets) {
		if (target->CheckLevel(level)) {
			try {
				// TODO
				target->Write(fmt);
			}
			catch (...) {
				std::fprintf(stderr, "Error on log write\n");
			}
		}
	}
	// unlock
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
