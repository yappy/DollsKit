#include "util.h"
#include <memory>
#include <cstring>
#include <sstream>
#include <algorithm>

namespace shanghai {
namespace util{

using namespace std::string_literals;

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

std::string OneLine(const std::string &str)
{
	size_t lf = str.find('\n');
	if (lf != std::string::npos) {
		return str.substr(0, lf);
	}
	else {
		return str;
	}
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

std::string DateTimeStr(std::time_t timestamp)
{
	struct tm local;
	char timecs[64] = "";

	::localtime_r(&timestamp, &local);
	if (std::strftime(timecs, sizeof(timecs), "%Y-%m-%d %T", &local) == 0) {
		timecs[0] = '\0';
	}

	return std::string(timecs);
}

// sample: "Thu Apr 06 15:24:15 +0000 2017"
std::time_t StrToTimeTwitter(const std::string &str)
{
	const std::array<const std::string, 12> mon_str = {
		"Jan"s, "Feb"s, "Mar"s, "Apr"s, "May"s, "Jun"s,
		"Jul"s, "Aug"s, "Sep"s, "Oct"s, "Nov"s, "Dec"s,
	};
	std::vector<std::string> tokens = Split(str, ' ');
	try {
		struct tm tm;
		std::memset(&tm, 0, sizeof(tm));
		{
			std::vector<std::string> timestr = Split(tokens.at(3), ':');
			tm.tm_hour = std::stoi(timestr.at(0));
			tm.tm_min = std::stoi(timestr.at(1));
			tm.tm_sec = std::stoi(timestr.at(2));
		}
		tm.tm_year = std::stoi(tokens.at(5)) - 1900;
		tm.tm_mon = std::distance(mon_str.begin(),
			std::find(mon_str.begin(), mon_str.end(), tokens.at(1)));
		tm.tm_mday = std::stoi(tokens.at(2));

		return ::timegm(&tm);
	}
	catch (std::exception &e) {
		throw std::runtime_error(e.what());
	}
}

std::string HtmlEscape(const std::string &src)
{
	std::string dst;
	dst.reserve(src.size());
	for (const char &c : src) {
		switch (c) {
		case '&':
			dst += "&amp;";
			break;
		case '"':
			dst += "&quot";
			break;
		case '\'':
			// since HTML5 spec!
			dst += "&apos";
			break;
		case '<':
			dst += "&lt;";
			break;
		case '>':
			dst += "&gt;";
			break;
		default:
			dst += c;
			break;
		}
	}
	return dst;
}

std::string UrlEncode(const std::string &src)
{
	std::string dst;
	const std::string special = "-._~";
	const char * const table = "0123456789ABCDEF";

	for (char c : src) {
		if (isalnum(c) || special.find_first_of(c) != std::string::npos) {
			dst += c;
		}
		else {
			uint32_t n = c;
			dst += '%';
			dst += table[(n >> 4) & 0xF];
			dst += table[n & 0xF];
		}
	}
	return dst;
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
