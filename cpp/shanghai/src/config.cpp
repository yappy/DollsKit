#include "config.h"
#include "util.h"

namespace shanghai {

void Config::Load(const std::string &file_name)
{
	std::string src;
	try {
		src = util::ReadStringFromFile(file_name);
	}
	catch (FileError &e) {
		throw ConfigError(e.what());
	}

	std::string err;
	json11::Json json = json11::Json::parse(src, err);
	if (json.is_null()) {
		throw ConfigError(err);
	}
	m_json = json;
}

// global instance
Config config;

}	// namespace shanghai
