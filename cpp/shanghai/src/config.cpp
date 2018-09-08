#include "config.h"
#include "util.h"

namespace shanghai {

namespace {

std::string CreateKeyName(std::initializer_list<const char *> keys)
{
	std::string result;
	bool is_first = true;
	for (const char *key : keys) {
		if (is_first) {
			is_first = false;
		}
		else {
			result += ".";
		}
		result += key;
	}
	return result;
}

}	// namespace

void Config::LoadString(const std::string &src)
{
	std::string err;
	json11::Json json = json11::Json::parse(src, err);
	if (json.is_null()) {
		throw ConfigError(err);
	}
	m_json = json;
}

void Config::LoadFile(const std::string &file_name)
{
	std::string src;
	try {
		src = util::ReadStringFromFile(file_name);
	}
	catch (FileError &e) {
		throw ConfigError(e.what());
	}
	LoadString(src);
}

bool Config::GetBool(std::initializer_list<const char *> keys)
{
	const json11::Json &value = GetValue(keys);
	if (!value.is_bool()) {
		throw ConfigError("Bool required: " + CreateKeyName(keys));
	}
	return value.bool_value();
}

int Config::GetInt(std::initializer_list<const char *> keys)
{
	const json11::Json &value = GetValue(keys);
	if (!value.is_number()) {
		throw ConfigError("Number required: " + CreateKeyName(keys));
	}
	return value.int_value();
}

std::string Config::GetStr(std::initializer_list<const char *> keys)
{
	const json11::Json &value = GetValue(keys);
	if (!value.is_string()) {
		throw ConfigError("String required: " + CreateKeyName(keys));
	}
	return value.string_value();
}

const json11::Json &Config::GetValue(std::initializer_list<const char *> keys)
{
	const json11::Json *cur = &m_json;
	for (const char *key : keys) {
		cur = &(*cur)[key];
	}
	return *cur;
}

// global instance
Config config;

}	// namespace shanghai
