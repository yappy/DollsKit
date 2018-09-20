#include "twitterconfig.h"
#include "../logger.h"
#include "../config.h"

namespace shanghai {
namespace system {

TwitterConfig::TwitterConfig()
{
	logger.Log(LogLevel::Info, "Initialize TwitterConfig...");

	ConsumerKey = config.GetStr({"Twitter", "ConsumerKey"});
	ConsumerSecret = config.GetStr({"Twitter", "ConsumerSecret"});
	AccessToken = config.GetStr({"Twitter", "AccessToken"});
	AccessSecret = config.GetStr({"Twitter", "AccessSecret"});

	logger.Log(LogLevel::Info, "Initialize TwitterConfig OK");
}

}	// namespace system
}	// namespace shanghai
