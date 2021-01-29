#include "discord.h"
#include "../logger.h"
#include "../util.h"
#include "../config.h"
#include <sleepy_discord/sleepy_discord.h>

namespace shanghai {
namespace system {

class Discord::MyClient : public SleepyDiscord::DiscordClient {
public:
	// コンストラクタ
	using SleepyDiscord::DiscordClient::DiscordClient;

protected:
	void onReady(SleepyDiscord::Ready ready) override
	{
		logger.Log(LogLevel::Info, "[Discord] Ready");
		{
			const SleepyDiscord::User &user = ready.user;
			const std::string &id = user.ID;
			logger.Log(LogLevel::Info, "[Discord] user %s %s bot:%s",
				id.c_str(), user.username.c_str(),
				user.bot ? "Yes" : "No");
		}
	}

	void onMessage(SleepyDiscord::Message message) override
	{
		if (message.isMentioned(getID())) {
			sendMessage(message.channelID, "はい");
		}
	}

	void onError(SleepyDiscord::ErrorCode errorCode,
		const std::string errorMessage) override
	{
		logger.Log(LogLevel::Error, "[Discord] %s", errorMessage.c_str());
	}
};

Discord::Discord()
{
	logger.Log(LogLevel::Info, "Initialize Discord...");

	bool enabled = config.GetBool({"Discord", "Enabled"});
	std::string token = config.GetStr({"Discord", "Token"});

	if (enabled) {
		m_client = std::make_unique<MyClient>(token, SleepyDiscord::USE_RUN_THREAD);
		m_thread = std::thread([this]() {
			try {
				m_client->run();
			}
			catch (std::exception &e) {
				logger.Log(LogLevel::Error, "Discord thread error: %s", e.what());
			}
		});
		logger.Log(LogLevel::Info, "Initialize Discord OK");
	}
	else {
		logger.Log(LogLevel::Info, "Initialize Discord OK (Disabled)");
	}
}

Discord::~Discord()
{
	logger.Log(LogLevel::Info, "Finalize Discord...");
	if (m_client != nullptr) {
		logger.Log(LogLevel::Info, "Quit asio...");
		logger.Flush();
		m_client->quit();

		logger.Log(LogLevel::Info, "Join discord thread...");
		logger.Flush();
		m_thread.join();
	}
	logger.Log(LogLevel::Info, "Finalize Discord OK");
}

}	// namespace system
}	// namespace shanghai
