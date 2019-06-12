#ifndef SHANGHAI_UTIL_H
#define SHANGHAI_UTIL_H

#include <stdexcept>
#include <system_error>
#include <initializer_list>
#include <limits>
#include <string>
#include <vector>
#include <ctime>

namespace shanghai {

class CancelError : public std::runtime_error {
public:
	CancelError(const char *msg) : runtime_error(msg) {}
	CancelError(const std::string &msg) : runtime_error(msg) {}
};

class FileError : public std::runtime_error {
public:
	FileError(const char *msg) : runtime_error(msg) {}
	FileError(const std::string &msg) : runtime_error(msg) {}
};

namespace util {

// 負の返り値の場合に errno から system_error を生成して投げる
template <typename R>
inline R SysCall(R ret)
{
	if (ret < 0) {
		throw std::system_error(errno, std::generic_category());
	}
	return ret;
}

inline int to_int(const std::string &str,
	int min = std::numeric_limits<int>::min(),
	int max = std::numeric_limits<int>::max())
{
	int n;
	try {
		n = std::stoi(str);
	}
	catch (std::logic_error &e) {
		throw std::runtime_error(e.what());
	}
	if (n < min) {
		throw std::overflow_error(
			str + " must less than " + std::to_string(min));
	}
	if (n > max) {
		throw std::overflow_error(
			str + " must greater than " + std::to_string(max));
	}
	return n;
}

inline uint64_t to_uint64(const std::string &str,
	uint64_t min = std::numeric_limits<uint64_t>::min(),
	uint64_t max = std::numeric_limits<uint64_t>::max())
{
	static_assert(sizeof(unsigned long long) == sizeof(uint64_t), "ull");

	uint64_t n;
	try {
		n = std::stoull(str);
	}
	catch (std::logic_error &e) {
		throw std::runtime_error(e.what());
	}
	if (n < min) {
		throw std::overflow_error(
			str + " must less than " + std::to_string(min));
	}
	if (n > max) {
		throw std::overflow_error(
			str + " must greater than " + std::to_string(max));
	}
	return n;
}

std::string ToString(const char *fmt, double d);
std::string Format(const char *fmt, std::initializer_list<std::string> args);
std::vector<std::string> Split(const std::string& input,
	char delim, bool remove_empty = false);
std::string ReplaceAll(const std::string &str,
	const std::string &from, const std::string &to);
std::string OneLine(const std::string &str);
std::string DateTimeStr(std::time_t timestamp = std::time(nullptr));
std::time_t StrToTimeTwitter(const std::string &str);
std::string HtmlEscape(const std::string &src);

std::vector<uint8_t> ReadFile(const std::string &file_name);
std::string ReadStringFromFile(const std::string &file_name);

}	// namespace util
}	// namespace shanghai

#endif	// SHANGHAI_UTIL_H
