#ifndef SHANGHAI_SYSTEM_TWITTER_H
#define SHANGHAI_SYSTEM_TWITTER_H

#include <string>

namespace shanghai {
namespace system {

class Twitter {
public:
	Twitter();
	~Twitter() = default;

	std::string ConsumerKey, ConsumerSecret, AccessToken, AccessSecret;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_TWITTER_H
