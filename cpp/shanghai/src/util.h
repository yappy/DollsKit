#ifndef SHANGHAI_UTIL_H
#define SHANGHAI_UTIL_H

#include <stdexcept>
#include <string>
#include <vector>

namespace shanghai {

class FileError : public std::runtime_error {
public:
	FileError(const char *msg) : runtime_error(msg) {}
	FileError(const std::string &msg) : runtime_error(msg) {}
};

namespace util {

std::vector<uint8_t> ReadFile(const std::string &file_name);
std::string ReadStringFromFile(const std::string &file_name);

}	// namespace util
}	// namespace shanghai

#endif	// SHANGHAI_UTIL_H
