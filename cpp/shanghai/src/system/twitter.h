#ifndef SHANGHAI_SYSTEM_TWITTER_H
#define SHANGHAI_SYSTEM_TWITTER_H

#include <json11.hpp>
#include <string>

namespace shanghai {
namespace system {

class Twitter final {
public:
	using Parameters = std::map<std::string, std::string>;

	Twitter();
	~Twitter() = default;

	uint64_t MyId() { return m_id; }
	// auto fake
	void Tweet(const std::string &msg);
	// auto fake, @user が本文中に必要
	void Tweet(const std::string &msg, const std::string &reply_to);

	json11::Json Statuses_Update(const Parameters &param = Parameters());
	json11::Json Statuses_HomeTimeline(const Parameters &param = Parameters());
	json11::Json Statuses_UserTimeline(const Parameters &param = Parameters());

private:
	bool m_fake_tweet;
	std::string m_consumer_key, m_consumer_secret;
	std::string m_access_token, m_access_secret;
	uint64_t m_id;

	json11::Json Account_VerifyCredentials(
		const Parameters &param = Parameters());

	json11::Json Request(const std::string &url, const std::string &method,
		const Parameters &param);
	json11::Json Get(const std::string &url, const Parameters &param);
	json11::Json Post(const std::string &url, const Parameters &param);
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_TWITTER_H
