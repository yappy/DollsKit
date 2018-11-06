#ifndef SHANGHAI_UTIL_H
#define SHANGHAI_UTIL_H

#include <stdexcept>
#include <system_error>
#include <initializer_list>
#include <string>
#include <vector>

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
inline int SysCall(R ret)
{
	if (ret < 0) {
		throw std::system_error(errno, std::generic_category());
	}
	return ret;
}

inline uint64_t to_uint64(const std::string &str)
{
	static_assert(sizeof(unsigned long long) == sizeof(uint64_t), "ull");
	return std::stoll(str);
}

std::string ToString(const char *fmt, double d);
std::string Format(const char *fmt, std::initializer_list<std::string> args);
std::vector<std::string> Split(const std::string& input,
	char delim, bool remove_empty = false);

std::vector<uint8_t> ReadFile(const std::string &file_name);
std::string ReadStringFromFile(const std::string &file_name);

}	// namespace util
}	// namespace shanghai

#endif	// SHANGHAI_UTIL_H
