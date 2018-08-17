#ifndef SHANGHAI_NET_H
#define SHANGHAI_NET_H

#include <stdexcept>
#include <vector>

namespace shanghai {

class NetworkError : public std::runtime_error {
public:
	NetworkError(const char *msg) : runtime_error(msg) {}
	NetworkError(const std::string &msg) : runtime_error(msg) {}
};

class Network final {
public:
	Network();
	~Network();
	Network(const Network &) = delete;
	Network & operator=(const Network &) = delete;

	std::vector<char> Download(const std::string &url);
};

extern Network net;

}	// namespace shanghai

#endif	// SHANGHAI_NET_H
