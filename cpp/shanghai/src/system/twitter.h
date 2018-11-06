#ifndef SHANGHAI_SYSTEM_TWITTER_H
#define SHANGHAI_SYSTEM_TWITTER_H

#include <json11.hpp>
#include <string>

namespace shanghai {
namespace system {

class Twitter {
public:
	Twitter();
	~Twitter() = default;

	json11::Json Statuses_HomeTimeline(int count = 20);

private:
	bool m_fake_tweet;
	std::string m_consumer_key, m_consumer_secret;
	std::string m_access_token, m_access_secret;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_TWITTER_H
