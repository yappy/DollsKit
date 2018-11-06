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

	m_fake_tweet = config.GetBool({"TwitterConfig", "FakeTweet"});
	m_consumer_key = config.GetStr({"TwitterConfig", "ConsumerKey"});
	m_consumer_secret = config.GetStr({"TwitterConfig", "ConsumerSecret"});
	m_access_token = config.GetStr({"TwitterConfig", "AccessToken"});
	m_access_secret = config.GetStr({"TwitterConfig", "AccessSecret"});

	logger.Log(LogLevel::Info, "Initialize Twitter OK");
}

json11::Json Twitter::Statuses_HomeTimeline(int count)
{
	std::string src = net.DownloadOAuth(
		URL_STATUSES_HOME_TIMELINE,
		"GET", {{ {"count", std::to_string(count)} }},
		m_consumer_key, m_access_token, m_consumer_secret, m_access_secret);
	std::string err;
	return json11::Json::parse(src, err);
}

}	// namespace system
}	// namespace shanghai
