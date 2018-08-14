#include "logger.h"
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <ctime>
#include <string>
#include <sys/stat.h>

namespace shanghai {

namespace {

using mtx_guard = std::lock_guard<std::mutex>;

const uint32_t MsgLenMax = 512;
const uint32_t LogLenMax = 1024;

// 標準出力の実装
class StdOutTarget : public LogTarget {
public:
	explicit StdOutTarget(LogLevel level) : LogTarget(level) {}
	virtual ~StdOutTarget() = default;

	virtual void Write(const char *str) override
	{
		std::puts(str);
	}
	virtual void Flush() override {}
};

class FileTarget : public LogTarget {
public:
	FileTarget(LogLevel level, const char *file_name,
		uint32_t rotate_size, uint32_t rotate_num) :
		LogTarget(level),
		m_file_name(file_name),
		m_rotate_size(rotate_size), m_rotate_num(rotate_num)
	{
		m_buffer.reserve(BufferSize);
	}
	virtual ~FileTarget() = default;

	virtual void Write(const char *str) override
	{
		if (m_buffer.size() + std::strlen(str) + 1 > BufferSize) {
			Flush();
		}
		m_buffer += str;
		m_buffer += '\n';
	}
	virtual void Flush() override
	{
		// 最初に空のバッファと交換しておく
		std::string data;
		data.swap(m_buffer);

		struct stat st;
		if (stat(m_file_name.c_str(), &st) == 0) {
			// このまま書くとローテーションサイズを超える場合
			if (st.st_size + data.size() > m_rotate_size) {
				std::string src;
				std::string dst;
				// 最後のを消す
				dst += m_file_name;
				dst += '.';
				dst += std::to_string(m_rotate_num - 1);
				std::remove(dst.c_str());
				// 1つずつ後ろにリネーム(上書きは処理系定義なので回避する)
				for (int i = m_rotate_num - 2; i >= 0; i--) {
					src.clear();
					dst.clear();
					src += m_file_name;
					if (i != 0) {
						src += '.';
						src += std::to_string(i);
					}
					dst += m_file_name;
					dst += '.';
					dst += std::to_string(i + 1);
					std::rename(src.c_str(), dst.c_str());
				}
			}
		}

		// 追記バイナリモード: UTF-8, LF
		FILE *fp = std::fopen(m_file_name.c_str(), "ab");
		if (fp != nullptr) {
			std::fwrite(data.c_str(), data.size(), 1, fp);
			std::fclose(fp);
		}
	}

private:
	static const int BufferSize = 64 * 1024;

	std::string m_file_name;
	uint32_t m_rotate_size;
	uint32_t m_rotate_num;
	std::string m_buffer;
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

		// timestr <- strftime()
		// ローカルタイムへの変換がスレッドセーフでないので一緒に排他する
		struct tm *local = std::localtime(&timestamp);
		if (std::strftime(timestr, sizeof(timestr), "%c", local) == 0) {
			timestr[0] = '\0';
		}
		// 1つの文字列にまとめる
		std::snprintf(logstr, sizeof(logstr) - 1,
			"%s [%s]: %s",
			timestr, LogLevelStr.at(static_cast<int>(level)), msg);
		logstr[sizeof(logstr) - 1] = '\0';

		for (auto &target : m_targets) {
			if (target->CheckLevel(level)) {
				try {
					target->Write(logstr);
				}
				catch (std::exception &e) {
					std::fprintf(stderr, "Error on log write: %s\n", e.what());
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
		catch (std::exception &e) {
			std::fprintf(stderr, "Error on log flush: %s\n", e.what());
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

void Logger::AddFile(LogLevel level,
	const char *file_name, uint32_t rotate_size, uint32_t rotate_num)
{
	mtx_guard lock(m_mtx);
	auto target = std::make_unique<FileTarget>(level, file_name,
		rotate_size, rotate_num);
	m_targets.push_back(std::move(target));
	// unlock
}

// global instance
Logger logger;

}	// namespace shanghai
