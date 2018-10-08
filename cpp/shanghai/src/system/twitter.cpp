#include "twitter.h"
#include "../logger.h"
#include "../config.h"

namespace shanghai {
namespace system {

Twitter::Twitter()
{
	logger.Log(LogLevel::Info, "Initialize Twitter...");

	ConsumerKey = config.GetStr({"TwitterConfig", "ConsumerKey"});
	ConsumerSecret = config.GetStr({"TwitterConfig", "ConsumerSecret"});
	AccessToken = config.GetStr({"TwitterConfig", "AccessToken"});
	AccessSecret = config.GetStr({"TwitterConfig", "AccessSecret"});

	logger.Log(LogLevel::Info, "Initialize Twitter OK");
}

}	// namespace system
}	// namespace shanghai
