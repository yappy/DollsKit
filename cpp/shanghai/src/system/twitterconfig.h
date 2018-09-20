#ifndef SHANGHAI_SYSTEM_TWITTERCONFIG_H
#define SHANGHAI_SYSTEM_TWITTERCONFIG_H

#include <string>

namespace shanghai {
namespace system {

class TwitterConfig {
public:
	TwitterConfig();
	~TwitterConfig() = default;

	std::string ConsumerKey, ConsumerSecret, AccessToken, AccessSecret;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_SYSTEM_H
