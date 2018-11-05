#include "twitter.h"
#include "../logger.h"
#include "../config.h"
#include "../net.h"

namespace shanghai {
namespace system {

namespace {
	const std::string URL_STATUSES = "https://api.twitter.com/1.1/statuses/"s;
	const std::string URL_STATUSES_HOME_TIMELINE =
		URL_STATUSES + "home_timeline.json";
}	// namespace

Twitter::Twitter()
{
	logger.Log(LogLevel::Info, "Initialize Twitter...");

	ConsumerKey = config.GetStr({"TwitterConfig", "ConsumerKey"});
	ConsumerSecret = config.GetStr({"TwitterConfig", "ConsumerSecret"});
	AccessToken = config.GetStr({"TwitterConfig", "AccessToken"});
	AccessSecret = config.GetStr({"TwitterConfig", "AccessSecret"});

	logger.Log(LogLevel::Info, "Initialize Twitter OK");
}

json11::Json Twitter::Statuses_HomeTimeline(int count)
{
	std::string src = net.DownloadOAuth(
		URL_STATUSES_HOME_TIMELINE,
		"GET", {{ {"count", std::to_string(count)} }},
		ConsumerKey, AccessToken, ConsumerSecret, AccessSecret);
	std::string err;
	return json11::Json::parse(src, err);
}

}	// namespace system
}	// namespace shanghai
