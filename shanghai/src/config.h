#ifndef SHANGHAI_CONFIG_H
#define SHANGHAI_CONFIG_H

#include <json11.hpp>
#include <stdexcept>
#include <string>
#include <deque>

namespace shanghai {

using namespace std::string_literals;

class ConfigError : public std::runtime_error {
public:
	ConfigError(const char *msg) : runtime_error(msg) {}
	ConfigError(const std::string &msg) : runtime_error(msg) {}
};

class Config final {
public:
	Config() = default;
	~Config() = default;

	void LoadString(const std::string &src);
	void LoadFile(const std::string &file_name);

	static std::string CreateKeyName(std::initializer_list<const char *> keys);
	bool GetBool(std::initializer_list<const char *> keys);
	int GetInt(std::initializer_list<const char *> keys);
	std::string GetStr(std::initializer_list<const char *> keys);
	std::vector<std::string> GetStrArray(
		std::initializer_list<const char *> keys);
	std::vector<std::pair<std::string, std::string>> GetStrPairArray(
		std::initializer_list<const char *> keys);

	// raw
	const json11::Json &GetValue(std::initializer_list<const char *> keys,
		size_t index = 0);

private:
	std::deque<json11::Json> m_json;
};

extern Config config;

}	// namespace shanghai

#endif	// SHANGHAI_CONFIG_H
