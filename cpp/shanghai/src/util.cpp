#include "util.h"
#include <memory>
#include <cstdio>
#include <sstream>

namespace shanghai {
namespace util{

std::string ToString(const char *fmt, double d)
{
	char buf[32];
	int n = std::snprintf(buf, sizeof(buf), fmt, d);
	if (n < 0 || n >= static_cast<int>(sizeof(buf))) {
		throw std::logic_error("snprintf failed");
	}
	return std::string(buf, n);
}

std::string Format(const char *fmt, std::initializer_list<std::string> args)
{
	std::string result = fmt;
	int num = 0;
	for (const auto &arg : args) {
		std::string target = "{";
		target += std::to_string(num);
		target += '}';
		std::string::size_type pos = 0;
		while ((pos = result.find(target, pos)) != std::string::npos) {
			result.replace(pos, target.size(), arg);
			pos += arg.size();
		}
		num++;
	}
	return result;
}

std::vector<std::string> Split(const std::string& input,
	char delim, bool remove_empty)
{
	std::istringstream stream(input);

	std::string elem;
	std::vector<std::string> result;
	while (std::getline(stream, elem, delim)) {
		if (!remove_empty || elem != "") {
			result.push_back(elem);
		}
	}
	return result;
}

std::string ReplaceAll(const std::string &str,
	const std::string &from, const std::string &to)
{
	if (from.empty()) {
		return str;
	}

	std::string result = str;
	size_t to_len = to.length();

	size_t pos = 0;
	while ((pos = result.find(from, pos)) != std::string::npos) {
		result.replace(pos, from.length(), to);
		pos += to_len;
	}
	return result;
}

namespace {

struct FileDeleter {
	void operator()(FILE *fp) {
		std::fclose(fp);
	}
};
using File = std::unique_ptr<FILE, FileDeleter>;

}	// namespace

std::vector<uint8_t> ReadFile(const std::string &file_name)
{
	File fp(std::fopen(file_name.c_str(), "rb"));
	if (fp == nullptr) {
		throw FileError("file open failed: " + file_name);
	}

	const size_t BufSize = 64 * 1024;
	std::vector<uint8_t> buf;
	while (1) {
		size_t org_size = buf.size();
		buf.resize(org_size + BufSize);
		size_t read_size = std::fread(buf.data() + org_size,
			1, BufSize, fp.get());
		if (std::ferror(fp.get())) {
			throw FileError("read file failed");
		}
		buf.resize(org_size + read_size);
		if (std::feof(fp.get())) {
			break;
		}
	}
	// move
	return buf;
}

std::string ReadStringFromFile(const std::string &file_name)
{
	std::vector<uint8_t> buf = ReadFile(file_name);
	return std::string(reinterpret_cast<char *>(buf.data()), buf.size());
}

}	// namespace util
}	// namespace shanghai
