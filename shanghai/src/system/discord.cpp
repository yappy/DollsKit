#include "discord.h"
#include "../logger.h"
#include "../util.h"
#include "../config.h"

namespace shanghai {
namespace system {

Discord::Discord()
{
	logger.Log(LogLevel::Info, "Initialize Discord...");

	bool enabled = config.GetBool({"Discord", "Enabled"});
	std::string token = config.GetStr({"Discord", "Token"});

	logger.Log(LogLevel::Info, "Initialize Discord OK");
}

}	// namespace system
}	// namespace shanghai
