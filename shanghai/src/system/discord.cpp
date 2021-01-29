#include "discord.h"
#include "../logger.h"
#include "../util.h"
#include "../config.h"
#include <sleepy_discord/sleepy_discord.h>

namespace shanghai {
namespace system {

namespace {

const std::string HELP_TEXT =
R"(/help
    Show this help
/server
    Show server list
/ch <server_id>
    Show channel list
)";

struct DiscordConfig {
	std::string DefaultReply = "";
};

}	// namespace

class Discord::MyClient : public SleepyDiscord::DiscordClient {
public:
	// コンストラクタ
	MyClient(const DiscordConfig &conf,
		const std::string &token, char numOfThreads)
		: SleepyDiscord::DiscordClient(token, numOfThreads),
		m_conf(conf)
	{}
	virtual ~MyClient() = default;

private:
	DiscordConfig m_conf;

	// コマンドとして処理出来たら true
	bool ExecuteCommand(SleepyDiscord::Snowflake<SleepyDiscord::Channel> ch,
		std::vector<std::string> args)
	{
		if (args.size() == 0) {
			return false;
		}
		if (args.at(0) == "/help") {
			sendMessage(ch, HELP_TEXT);
			return true;
		}
		else if (args.at(0) == "/server") {
			std::vector<SleepyDiscord::Server> resp = getServers();
			std::string msg = util::Format("{0} Server(s)",
				{std::to_string(resp.size())});
			for (const auto &server : resp) {
				msg += '\n';
				msg += server.ID;
				msg += ' ';
				msg += server.name;
			}
			sendMessage(ch, msg);
			return true;
		}
		else if (args.at(0) == "/ch") {
			if (args.size() < 2) {
				sendMessage(ch, "Argument error.");
				return true;
			}
			std::vector<SleepyDiscord::Channel> resp =
				getServerChannels(args.at(1));
			std::string msg = util::Format("{0} Channels(s)",
				{std::to_string(resp.size())});
			for (const auto &ch : resp) {
				if (ch.type != SleepyDiscord::Channel::ChannelType::SERVER_TEXT) {
					continue;
				}
				msg += '\n';
				msg += ch.ID;
				msg += ' ';
				msg += ch.name;
			}
			sendMessage(ch, msg);
			return true;
		}
		else {
			return false;
		}
	}

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
		// ミラーマッチ対策として bot には反応しないようにする
		if (message.author.bot) {
			return;
		}

		logger.Log(LogLevel::Info, "[Discord] Message");
		logger.Log(LogLevel::Info, "[Discord] %s", message.content.c_str());
		// メンション時のみでフィルタ
		if (message.isMentioned(getID())) {
			// 半角スペースで区切ってメンションを削除
			// 例: <@!123456789>
			std::vector<std::string> tokens = util::Split(
				message.content, ' ', true);
			auto result = std::remove_if(tokens.begin(), tokens.end(),
				[](const std::string &s) {
					return s.find("<") == 0 && s.rfind(">") == s.size() - 1;
				});
			tokens.erase(result, tokens.end());

			// コマンドとして実行
			// できなかったらデフォルト返信
			if (!ExecuteCommand(message.channelID, tokens)) {
				sendMessage(message.channelID, m_conf.DefaultReply);
			}
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
		DiscordConfig dconf;
		dconf.DefaultReply = config.GetStr({"Discord", "DefaultReply"});

		m_client = std::make_unique<MyClient>(
			dconf, token, SleepyDiscord::USE_RUN_THREAD);
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
