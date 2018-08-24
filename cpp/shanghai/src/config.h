#ifndef SHANGHAI_CONFIG_H
#define SHANGHAI_CONFIG_H

#include <json11.hpp>
#include <stdexcept>
#include <string>

namespace shanghai {

class ConfigError : public std::runtime_error {
public:
	ConfigError(const char *msg) : runtime_error(msg) {}
	ConfigError(const std::string &msg) : runtime_error(msg) {}
};

class Config final {
public:
	Config() = default;
	~Config() = default;

	void Load(const std::string &file_name);

private:
	json11::Json m_json;
};

extern Config config;

}	// namespace shanghai

#endif	// SHANGHAI_CONFIG_H
