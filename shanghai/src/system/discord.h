#ifndef SHANGHAI_SYSTEM_DISCORD_H
#define SHANGHAI_SYSTEM_DISCORD_H

#include <memory>
#include <thread>

namespace shanghai {
namespace system {

class Discord final {
public:
	Discord();
	~Discord();

private:
	class MyClient;
	std::thread m_thread;
	std::unique_ptr<MyClient> m_client;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_DISCORD_H
