#ifndef SHANGHAI_SYSTEM_DISCORD_H
#define SHANGHAI_SYSTEM_DISCORD_H

#include <memory>
#include <thread>

namespace shanghai {
namespace system {

class MyDiscordClient;

class Discord final {
public:
	Discord();
	~Discord();

	void Send(const std::string &text) noexcept;

private:
	std::thread m_thread;
	std::unique_ptr<MyDiscordClient> m_client;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_DISCORD_H
