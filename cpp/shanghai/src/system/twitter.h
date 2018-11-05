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

	std::string ConsumerKey, ConsumerSecret, AccessToken, AccessSecret;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_TWITTER_H
