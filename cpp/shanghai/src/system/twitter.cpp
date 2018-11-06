#include "twitter.h"
#include "../logger.h"
#include "../util.h"
#include "../config.h"
#include "../net.h"

namespace shanghai {
namespace system {

namespace {
	const std::string URL_ACCOUNT = "https://api.twitter.com/1.1/account/"s;
	const std::string URL_ACCOUNT_VERIFY_CREDENTIALS =
		URL_ACCOUNT + "verify_credentials.json";

	const std::string URL_STATUSES = "https://api.twitter.com/1.1/statuses/"s;
	const std::string URL_STATUSES_UPDATE =
		URL_STATUSES + "update.json";
	const std::string URL_STATUSES_HOME_TIMELINE =
		URL_STATUSES + "home_timeline.json";
	const std::string URL_STATUSES_USER_TIMELINE =
		URL_STATUSES + "user_timeline.json";
}	// namespace

Twitter::Twitter()
{
	logger.Log(LogLevel::Info, "Initialize Twitter...");

	m_fake_tweet = config.GetBool({"TwitterConfig", "FakeTweet"});
	m_consumer_key = config.GetStr({"TwitterConfig", "ConsumerKey"});
	m_consumer_secret = config.GetStr({"TwitterConfig", "ConsumerSecret"});
	m_access_token = config.GetStr({"TwitterConfig", "AccessToken"});
	m_access_secret = config.GetStr({"TwitterConfig", "AccessSecret"});

	json11::Json cred_result = Account_VerifyCredentials();
	m_id = util::to_uint64(cred_result["id_str"].string_value());
	logger.Log(LogLevel::Info, "Verify credentials OK: id=%" PRIu64, m_id);

	logger.Log(LogLevel::Info, "Initialize Twitter OK");
}

void Twitter::Tweet(const std::string &msg)
{
	if (!m_fake_tweet) {
		Statuses_Update({{"status", msg}});
	}
	else {
		logger.Log(LogLevel::Info, "Fake Tweet: %s", msg.c_str());
	}
}

json11::Json Twitter::Statuses_Update(const Parameters &param)
{
	return Post(URL_STATUSES_UPDATE, param);
}

json11::Json Twitter::Statuses_HomeTimeline(const Parameters &param)
{
	return Get(URL_STATUSES_HOME_TIMELINE, param);
}

json11::Json Twitter::Statuses_UserTimeline(const Parameters &param)
{
	return Get(URL_STATUSES_USER_TIMELINE, param);
}

json11::Json Twitter::Account_VerifyCredentials(const Parameters &param)
{
	return Get(URL_ACCOUNT_VERIFY_CREDENTIALS, param);
}

json11::Json Twitter::Request(const std::string &url, const std::string &method,
	const Parameters &param)
{
	std::string src = net.DownloadOAuth(
		url, method, param,
		m_consumer_key, m_access_token, m_consumer_secret, m_access_secret);
	std::string err;
	return json11::Json::parse(src, err);
}

json11::Json Twitter::Get(const std::string &url, const Parameters &param)
{
	return Request(url, "GET"s, param);
}

json11::Json Twitter::Post(const std::string &url, const Parameters &param)
{
	return Request(url, "POST"s, param);
}

}	// namespace system
}	// namespace shanghai
