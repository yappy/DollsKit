#ifndef SHANGHAI_CONFIG_H
#define SHANGHAI_CONFIG_H

#include <json11.hpp>
#include <stdexcept>
#include <string>

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

	bool GetBool(std::initializer_list<const char *> keys);
	int GetInt(std::initializer_list<const char *> keys);
	std::string GetStr(std::initializer_list<const char *> keys);

private:
	json11::Json m_json;

	const json11::Json &GetValue(std::initializer_list<const char *> keys);
};

extern Config config;

}	// namespace shanghai

#endif	// SHANGHAI_CONFIG_H
