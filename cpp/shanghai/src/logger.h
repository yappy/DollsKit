#ifndef SHANGHAI_LOGGER_H
#define SHANGHAI_LOGGER_H

#include <memory>
#include <mutex>
#include <vector>

namespace shanghai {

/*
 * ログレベル
 * 上のものほど深刻
 */
enum class LogLevel {
	Fatal,
	Error,
	Warning,
	Info,
	Trace,
};

class LogTarget {
protected:
	explicit LogTarget(LogLevel level) : m_level(level) {}

public:
	virtual ~LogTarget() = default;

	bool CheckLevel(LogLevel level);
	virtual void Write(const char *str, LogLevel level, uintptr_t tid,
		const char *timestamp) = 0;
	virtual void Flush() = 0;

private:
	LogLevel m_level;
};

/*
 * ログシステム
 */
class Logger final {
public:
	Logger() = default;
	// デストラクタでもフラッシュする
	~Logger();

	void Log(const char *fmt, ...);
	void Flush();

	// 出力先に標準出力を追加する (buffering on)
	void AddStdOut(LogLevel level);
	// 出力先にファイルを追加する (buffering off)
	void AddFile(LogLevel level,
		const char *file_name = "log%d.txt",
		uint32_t rotate_size = 10 * 1024 * 1024,
		uint32_t rotate_num = 2);

private:
	std::mutex m_mtx;
	std::vector<std::unique_ptr<LogTarget>> m_targets;
};

}	// namespace shanghai

#endif	// SHANGHAI_LOGGER_H
