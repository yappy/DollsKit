#include "config.h"
#include "util.h"

namespace shanghai {

void Config::LoadString(const std::string &src)
{
	std::string err;
	json11::Json json = json11::Json::parse(src, err);
	if (json.is_null()) {
		throw ConfigError(err);
	}
	m_json.emplace_front(std::move(json));
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

std::string Config::CreateKeyName(std::initializer_list<const char *> keys)
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

std::vector<std::string> Config::GetStrArray(
	std::initializer_list<const char *> keys)
{
	const json11::Json &value = GetValue(keys);
	if (!value.is_array()) {
		throw ConfigError("String array required: " + CreateKeyName(keys));
	}

	std::vector<std::string> result;
	const std::vector<json11::Json> &array = value.array_items();
	for (const auto &item : array) {
		if (!item.is_string()) {
			throw ConfigError("String required: " + CreateKeyName(keys));
		}
		result.emplace_back(item.string_value());
	}
	return result;
}

std::vector<std::pair<std::string, std::string>> Config::GetStrPairArray(
	std::initializer_list<const char *> keys)
{
	const json11::Json &value = GetValue(keys);
	if (!value.is_array()) {
		throw ConfigError("String array required: " + CreateKeyName(keys));
	}

	std::vector<std::pair<std::string, std::string>> result;
	const std::vector<json11::Json> &array = value.array_items();
	for (const auto &item : array) {
		if (!item.is_array() || !item[0].is_string() || !item[1].is_string()) {
			throw ConfigError("String pair required: " + CreateKeyName(keys));
		}
		result.emplace_back(item[0].string_value(), item[1].string_value());
	}
	return result;
}

const json11::Json &Config::GetValue(std::initializer_list<const char *> keys,
	size_t index)
{
	if (index >= m_json.size()) {
		// return static null
		return json11::Json()[0];
	}
	const json11::Json *cur = &m_json.at(index);
	for (const char *key : keys) {
		cur = &(*cur)[key];
	}
	if (!cur->is_null()) {
		return *cur;
	}
	else {
		return GetValue(keys, index + 1);
	}
}

// global instance
Config config;

}	// namespace shanghai
