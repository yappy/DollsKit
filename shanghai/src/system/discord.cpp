#include "discord.h"
#include "system.h"
#include "../logger.h"
#include "../util.h"
#include "../config.h"
#include <sleepy_discord/sleepy_discord.h>
#include <random>
#include <map>

namespace shanghai {
namespace system {

namespace {

const std::string HELP_TEXT =
R"(/help
    Show this help
/info
    Print system information
/server
    Show server list
/ch <server_id>
    Show channel list
/dice [<max>] [<times>]
    Nondeterministic dice roll
/haipai
    Deal piles (MT19937)
)";

struct DiscordConfig {
	std::string DefaultReply = "";
};

template <class F>
inline void CallNoExcept(F f) noexcept
{
	try {
		f();
	}
	catch (std::exception &e) {
		logger.Log(LogLevel::Error, "[Discord] Error in handler: %s", e.what());
	}
	catch (...) {
		logger.Log(LogLevel::Error, "[Discord] Unknown error in handler");
	}
}

}	// namespace

class MyDiscordClient : public SleepyDiscord::DiscordClient {
public:
	// コンストラクタ
	MyDiscordClient(const DiscordConfig &conf,
		const std::string &token, char numOfThreads)
		: SleepyDiscord::DiscordClient(token, numOfThreads),
		m_conf(conf)
	{
		RegisterCommands();
	}
	virtual ~MyDiscordClient() = default;

private:
	DiscordConfig m_conf;
	// 非決定論的乱数生成器 (連打禁止)
	std::random_device m_rng;

	using Ch = SleepyDiscord::Snowflake<SleepyDiscord::Channel>;
	using CmdFunc = void(Ch ch, const std::vector<std::string> &args);
	std::unordered_map<std::string, std::function<CmdFunc>> m_cmdmap;

	// イベントハンドラ本処理 (例外送出あり)
	void DoOnReady(SleepyDiscord::Ready &ready);
	void DoOnMessage(SleepyDiscord::Message &message);
	void DoOnError(SleepyDiscord::ErrorCode errorCode,
		const std::string &errorMessage);

	// 全コマンドを string->func ハッシュテーブルに登録する
	void RegisterCommands();
	// メンション時の OnMessage 処理
	// コマンドとして処理出来たら true
	bool ExecuteCommand(Ch ch, const std::vector<std::string> &args);

	// コマンドとして登録する関数
	void CmdHelp(Ch ch, const std::vector<std::string> &args);
	void CmdInfo(Ch ch, const std::vector<std::string> &args);
	void CmdServer(Ch ch, const std::vector<std::string> &args);
	void CmdCh(Ch ch, const std::vector<std::string> &args);
	void CmdDice(Ch ch, const std::vector<std::string> &args);
	void CmdHaipai(Ch ch, const std::vector<std::string> &args);

protected:
	// イベントハンドラ override
	// 例外送出するとスレッドによってクラッシュする場合があるので
	// 外に漏らさないようにしつつ private 関数を呼ぶ
	void onReady(SleepyDiscord::Ready ready) noexcept override
	{
		CallNoExcept(std::bind(&MyDiscordClient::DoOnReady, this, ready));
	}
	void onMessage(SleepyDiscord::Message message) noexcept override
	{
		CallNoExcept(std::bind(&MyDiscordClient::DoOnMessage, this, message));
	}
	void onError(SleepyDiscord::ErrorCode errorCode,
		const std::string errorMessage) noexcept override
	{
		CallNoExcept(std::bind(&MyDiscordClient::DoOnError,
			this, errorCode, errorMessage));
	}
};

void MyDiscordClient::DoOnReady(SleepyDiscord::Ready &ready)
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

void MyDiscordClient::DoOnMessage(SleepyDiscord::Message &message)
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
			std::string msg = m_conf.DefaultReply;
			msg += "\n(Help command: /help)";
			sendMessage(message.channelID, msg);
		}
	}
}

void MyDiscordClient::DoOnError(SleepyDiscord::ErrorCode errorCode,
	const std::string &errorMessage)
{
	logger.Log(LogLevel::Error, "[Discord] %s", errorMessage.c_str());
}

void MyDiscordClient::RegisterCommands()
{
	using namespace std::placeholders;
	m_cmdmap.emplace("/help",
		std::bind(&MyDiscordClient::CmdHelp, this, _1, _2));
	m_cmdmap.emplace("/info",
		std::bind(&MyDiscordClient::CmdInfo, this, _1, _2));
	m_cmdmap.emplace("/server",
		std::bind(&MyDiscordClient::CmdServer, this, _1, _2));
	m_cmdmap.emplace("/ch",
		std::bind(&MyDiscordClient::CmdCh, this, _1, _2));
	m_cmdmap.emplace("/dice",
		std::bind(&MyDiscordClient::CmdDice, this, _1, _2));
	m_cmdmap.emplace("/haipai",
		std::bind(&MyDiscordClient::CmdHaipai, this, _1, _2));
}

bool MyDiscordClient::ExecuteCommand(
	SleepyDiscord::Snowflake<SleepyDiscord::Channel> ch,
	const std::vector<std::string> &args)
{
	if (args.size() == 0) {
		return false;
	}
	auto it = m_cmdmap.find(args.at(0));
	if (it != m_cmdmap.end()) {
		it->second(ch, args);
		return true;
	}
	else {
		return false;
	}
}

void MyDiscordClient::CmdHelp(Ch ch, const std::vector<std::string> &args)
{
	sendMessage(ch, HELP_TEXT);
}

void MyDiscordClient::CmdInfo(Ch ch, const std::vector<std::string> &args)
{
	auto &sys_info = system::Get().sys_info;
	system::SysInfoData data = sys_info.Get();
	std::string msg = util::Format(
		"Build Type: {0}\n"
		"Branch: {1}\n"
		"Commit: {2}\n"
		"White: {3}\n"
		"Black: {4}",
		{
			data.build_type, data.git_branch, data.git_hash,
			std::to_string(data.white), std::to_string(data.black)
		});
	sendMessage(ch, msg);
}

void MyDiscordClient::CmdServer(Ch ch, const std::vector<std::string> &args)
{
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
}

void MyDiscordClient::CmdCh(Ch ch, const std::vector<std::string> &args)
{
	if (args.size() < 2) {
		sendMessage(ch, "Argument error.");
		return;
	}
	std::vector<SleepyDiscord::Channel> resp =
		getServerChannels(args.at(1));
	std::string msg = util::Format("Channel(s)",
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
}

void MyDiscordClient::CmdDice(Ch ch, const std::vector<std::string> &args)
{
	const uint64_t DICE_MAX = 1ULL << 56;
	const uint64_t COUNT_MAX = 100;
	//     d * c < U64
	// <=> d < U64 / c
	static_assert(
		DICE_MAX <
		std::numeric_limits<uint64_t>::max() / COUNT_MAX);

	uint64_t d = 6;
	uint64_t count = 1;
	bool error = false;
	if (args.size() >= 2) {
		try {
			d = util::to_uint64(args.at(1), 1, DICE_MAX);
		}
		catch(...){
			error = true;
		}
	}
	if (args.size() >= 3) {
		try {
			count = util::to_uint64(args.at(2), 1, COUNT_MAX);
		}
		catch(...){
			error = true;
		}
	}
	if (error) {
		std::string msg = util::Format(
			"1 <= DICE <= {0}\n"
			"1 <= COUNT <= {1}",
			{std::to_string(DICE_MAX), std::to_string(COUNT_MAX)});
		sendMessage(ch, msg);
		return;
	}

	std::string seq = "";
	uint64_t sum = 0;
	for (uint64_t i = 0; i < count; i++) {
		std::uniform_int_distribution<uint64_t> dist(1, d);
		uint64_t r = dist(m_rng);
		sum += r;
		if (seq.size() != 0) {
			seq += ", ";
		}
		seq += std::to_string(r);
	}
	std::string msg;
	if (count == 1) {
		msg = std::to_string(sum);
	} else {
		msg = util::Format("{0}\n({1})", {std::to_string(sum), seq});
	}
	sendMessage(ch, msg);
}

void MyDiscordClient::CmdHaipai(Ch ch, const std::vector<std::string> &args)
{
	// 文字コード順だと
	// 🀀🀁🀂🀃🀄🀅🀆🀇🀈🀉🀊🀋🀌🀍🀏🀏🀐🀑🀒🀓🀕🀕🀖🀗🀘🀙🀚🀛🀜🀝🀞🀟🀠🀡
	// になってしまう
	const char RES[] = u8"🀇🀈🀉🀊🀋🀌🀍🀏🀏🀙🀚🀛🀜🀝🀞🀟🀠🀡🀐🀑🀒🀓🀕🀕🀖🀗🀘🀀🀁🀂🀃🀆🀅🀄";
	// sizeof(emoji_hai) == 4
	static_assert(sizeof(RES) == 4 * 34 + 1);

	std::array<int, 136> deck;
	for (int i = 0; i < 34; i++) {
		deck[i * 4 + 0] = deck[i * 4 + 1] =
		deck[i * 4 + 2] = deck[i * 4 + 3] = i;
	}
	std::mt19937 engine(m_rng());
	std::shuffle(deck.begin(), deck.end(), engine);
	std::sort(deck.begin(), deck.begin() + 14);
	std::string msg;
	for (int i = 0; i < 14; i++) {
		int x = deck.at(i);
		msg += std::string(RES, x * 4, 4);
	}
	sendMessage(ch, msg);
}

Discord::Discord()
{
	logger.Log(LogLevel::Info, "Initialize Discord...");

	bool enabled = config.GetBool({"Discord", "Enabled"});
	std::string token = config.GetStr({"Discord", "Token"});

	if (enabled) {
		DiscordConfig dconf;
		dconf.DefaultReply = config.GetStr({"Discord", "DefaultReply"});

		m_client = std::make_unique<MyDiscordClient>(
			dconf, token, SleepyDiscord::USER_CONTROLED_THREADS);
		m_thread = std::thread([this]() {
			bool retry = false;
			do {
				try {
					logger.Log(LogLevel::Info, "Discord client run");
					m_client->run();
					logger.Log(LogLevel::Info, "Discord client run returned");
				}
				catch (std::exception &e) {
					logger.Log(LogLevel::Error, "Discord thread error: %s", e.what());
					retry = true;
				}
				catch (...) {
					logger.Log(LogLevel::Error, "Discord thread unknown error");
					retry = true;
				}
				if (retry) {
					std::this_thread::sleep_for(std::chrono::seconds(10));
				}
			} while (retry);
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
