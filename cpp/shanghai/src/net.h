#ifndef SHANGHAI_NET_H
#define SHANGHAI_NET_H

#include <stdexcept>
#include <atomic>
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

	// 完了するまでブロックする
	// タイムアウトは 0 で無限待ち
	std::vector<char> Download(const std::string &url, int timeout_sec = 0,
		const std::atomic<bool> &cancel = std::atomic<bool>(false));
};

extern Network net;

}	// namespace shanghai

#endif	// SHANGHAI_NET_H
