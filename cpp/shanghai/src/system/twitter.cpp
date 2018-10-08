#include "twitter.h"
#include "../logger.h"
#include "../config.h"

namespace shanghai {
namespace system {

Twitter::Twitter()
{
	logger.Log(LogLevel::Info, "Initialize Twitter...");

	ConsumerKey = config.GetStr({"Twitter", "ConsumerKey"});
	ConsumerSecret = config.GetStr({"Twitter", "ConsumerSecret"});
	AccessToken = config.GetStr({"Twitter", "AccessToken"});
	AccessSecret = config.GetStr({"Twitter", "AccessSecret"});

	logger.Log(LogLevel::Info, "Initialize Twitter OK");
}

}	// namespace system
}	// namespace shanghai
