#include "util.h"
#include <memory>
#include <cstdio>

namespace shanghai {
namespace util{

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
