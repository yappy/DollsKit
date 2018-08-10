#ifndef SHANGHAI_LOGGER_H
#define SHANGHAI_LOGGER_H

#include <memory>
#include <mutex>
#include <array>
#include <vector>

namespace shanghai {

/*
 * ログレベル
 * 上のものほど深刻
 */
enum class LogLevel {
	Fatal,
	Error,
	Warn,
	Info,
	Trace,

	Count
};

const std::array<const char *, static_cast<int>(LogLevel::Count)>
LogLevelStr = {
	"Fatal",
	"Error",
	"Warn",
	"Info",
	"Trace",
};

/*
 * ログの出力先
 */
class LogTarget {
protected:
	explicit LogTarget(LogLevel level) : m_level(level) {}

public:
	virtual ~LogTarget() = default;

	// コンストラクタで指定したフィルタレベルを満たすかを返す
	bool CheckLevel(LogLevel level) noexcept
	{
		return static_cast<int>(level) < static_cast<int>(m_level);
	}

	// 1エントリを書き込む (他の呼び出しとは排他)
	virtual void Write(const char *str) = 0;
	// バッファリングしている場合はフラッシュする (他の呼び出しとは排他)
	virtual void Flush() = 0;

private:
	LogLevel m_level;
};

/*
 * ログシステム
 * thread safe
 */
class Logger final {
public:
	Logger() = default;
	// フラッシュした後メンバを解放する
	~Logger()
	{
		Flush();
	}

	// ログを出す
	void Log(LogLevel level, const char *fmt, ...) noexcept
		__attribute__((format(printf, 3, 4)));
	// バッファリングされている出力先をフラッシュする
	void Flush() noexcept;

	// 出力先に標準出力を追加する (buffering off)
	void AddStdOut(LogLevel level);
	// 出力先にファイルを追加する (buffering on)
	void AddFile(LogLevel level,
		const char *file_name = "shanghai.log",
		uint32_t rotate_size = 10 * 1024 * 1024,
		uint32_t rotate_num = 2);

private:
	std::mutex m_mtx;
	std::vector<std::unique_ptr<LogTarget>> m_targets;
};

/*
 * グローバルロガー
 */
extern std::unique_ptr<Logger> logger;

}	// namespace shanghai

#endif	// SHANGHAI_LOGGER_H
