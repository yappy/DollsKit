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
/user <server_id>
    Show member list (<=1000 only)
    To enable this, Bot settings > "Privileged Gateway Intents" > "Server Members Intent"
/play <message>
    Change playing game
/dice [<max>] [<times>]
    Nondeterministic dice roll
/haipai
    Deal piles (MT19937)
)";

struct DiscordConfig {
	std::string DefaultReply = "";
	std::string NotifChannel = "";
	std::vector<std::string> PrivilegedUsers;
	std::string DenyMessage = "";

	bool HasPrivilege(std::string user)
	{
		return std::find(PrivilegedUsers.begin(), PrivilegedUsers.end(), user)
			!= PrivilegedUsers.end();
	}
};

template <class F>
inline void CallNoExcept(F f) noexcept
{
	try {
		f();
	}
	catch (std::exception &e) {
		logger.Log(LogLevel::Error, "[Discord] Error: %s", e.what());
	}
	catch (...) {
		logger.Log(LogLevel::Error, "[Discord] Unknown error");
	}
}

}	// namespace

class MyDiscordClient : public SleepyDiscord::DiscordClient {
public:
	MyDiscordClient(const DiscordConfig &conf,
		const std::string &token, char numOfThreads)
		: SleepyDiscord::DiscordClient(token, numOfThreads),
		m_conf(conf)
	{
		RegisterCommands();
	}
	virtual ~MyDiscordClient() = default;

	void SendMessage(const std::string &text) noexcept;

private:
	DiscordConfig m_conf;
	// 非決定論的乱数生成器 (連打禁止)
	std::random_device m_rng;

	using Msg = SleepyDiscord::Message;
	using CmdFunc = void(Msg msg, const std::vector<std::string> &args);
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
	bool ExecuteCommand(Msg msg, const std::vector<std::string> &args);

	// コマンドとして登録する関数
	void CmdHelp(Msg msg, const std::vector<std::string> &args);
	void CmdInfo(Msg msg, const std::vector<std::string> &args);
	void CmdServer(Msg msg, const std::vector<std::string> &args);
	void CmdCh(Msg msg, const std::vector<std::string> &args);
	void CmdUser(Msg msg, const std::vector<std::string> &args);
	void CmdPlay(Msg msg, const std::vector<std::string> &args);
	void CmdDice(Msg msg, const std::vector<std::string> &args);
	void CmdHaipai(Msg msg, const std::vector<std::string> &args);

	void SendLargeMessage(const std::string &chid,
		const std::vector<std::string> &lines);

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

void MyDiscordClient::SendMessage(const std::string &text) noexcept
{
	auto f = [this, &text]() {
		if (m_conf.NotifChannel != "") {
			logger.Log(LogLevel::Info, "Discord Notif: %s", text.c_str());
			sendMessage(m_conf.NotifChannel, text);
		}
		else {
			logger.Log(LogLevel::Info, "Discord Notif (disabled): %s",
				text.c_str());
		}
	};
	CallNoExcept(f);
}

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
		if (!ExecuteCommand(message, tokens)) {
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
	m_cmdmap.emplace("/user",
		std::bind(&MyDiscordClient::CmdUser, this, _1, _2));
	m_cmdmap.emplace("/play",
		std::bind(&MyDiscordClient::CmdPlay, this, _1, _2));
	m_cmdmap.emplace("/dice",
		std::bind(&MyDiscordClient::CmdDice, this, _1, _2));
	m_cmdmap.emplace("/haipai",
		std::bind(&MyDiscordClient::CmdHaipai, this, _1, _2));
}

bool MyDiscordClient::ExecuteCommand(
	Msg msg, const std::vector<std::string> &args)
{
	if (args.size() == 0) {
		return false;
	}
	auto it = m_cmdmap.find(args.at(0));
	if (it != m_cmdmap.end()) {
		it->second(msg, args);
		return true;
	}
	else {
		return false;
	}
}

void MyDiscordClient::CmdHelp(Msg msg, const std::vector<std::string> &args)
{
	sendMessage(msg.channelID, HELP_TEXT);
}

void MyDiscordClient::CmdInfo(Msg msg, const std::vector<std::string> &args)
{
	auto &sys_info = system::Get().sys_info;
	system::SysInfoData data = sys_info.Get();
	std::string text = util::Format(
		"Build Type: {0}\n"
		"Branch: {1}\n"
		"Commit: {2}\n"
		"Date: {3}\n"
		"White: {4}\n"
		"Black: {5}",
		{
			data.build_type, data.git_branch, data.git_hash, data.git_date,
			std::to_string(data.white), std::to_string(data.black)
		});
	sendMessage(msg.channelID, text);
}

void MyDiscordClient::CmdServer(Msg msg, const std::vector<std::string> &args)
{
	std::vector<SleepyDiscord::Server> resp = getServers();
	std::string text = util::Format("{0} Server(s)",
		{std::to_string(resp.size())});
	for (const auto &server : resp) {
		text += '\n';
		text += server.ID;
		text += ' ';
		text += server.name;
	}
	sendMessage(msg.channelID, text);
}

void MyDiscordClient::CmdCh(Msg msg, const std::vector<std::string> &args)
{
	if (args.size() < 2) {
		sendMessage(msg.channelID, "Argument error.");
		return;
	}
	std::vector<SleepyDiscord::Channel> resp =
		getServerChannels(args.at(1));
	std::string text = util::Format("{0} Channel(s)",
		{std::to_string(resp.size())});
	for (const auto &ch : resp) {
		if (ch.type != SleepyDiscord::Channel::ChannelType::SERVER_TEXT) {
			continue;
		}
		text += '\n';
		text += ch.ID;
		text += ' ';
		text += ch.name;
	}
	sendMessage(msg.channelID, text);
}

void MyDiscordClient::CmdUser(Msg msg, const std::vector<std::string> &args)
{
	if (args.size() < 2) {
		sendMessage(msg.channelID, "Argument error.");
		return;
	}

	// 最大 1000 件まで
	// それ以上は複数回に分ける必要がある(未対応)
	std::vector<SleepyDiscord::ServerMember> resp =
		listMembers(args.at(1), 1000);

	std::vector<std::string> lines;
	lines.emplace_back(util::Format("{0} User(s)",
		{std::to_string(resp.size())}));
	for (const auto &member : resp) {
		const SleepyDiscord::User &user = member.user;
		std::string line;
		line += user.ID;
		line += ' ';
		line += user.username;
		line += user.bot ? " [BOT]" : "";
		lines.emplace_back(std::move(line));
	}

	SendLargeMessage(msg.channelID, lines);
}

void MyDiscordClient::CmdPlay(Msg msg, const std::vector<std::string> &args)
{
	if (!m_conf.HasPrivilege(msg.author.ID)) {
		sendMessage(msg.channelID, m_conf.DenyMessage);
		return;
	}
	if (args.size() < 2) {
		sendMessage(msg.channelID, "Argument error.");
		return;
	}

	std::string game = args.at(1);
	updateStatus(game);
	sendMessage(msg.channelID, util::Format("Now playing: {0}", {game}));
}

void MyDiscordClient::CmdDice(Msg msg, const std::vector<std::string> &args)
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
		std::string text = util::Format(
			"1 <= DICE <= {0}\n"
			"1 <= COUNT <= {1}",
			{std::to_string(DICE_MAX), std::to_string(COUNT_MAX)});
		sendMessage(msg.channelID, text);
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
	std::string text;
	if (count == 1) {
		text = std::to_string(sum);
	} else {
		text = util::Format("{0}\n({1})", {std::to_string(sum), seq});
	}
	sendMessage(msg.channelID, text);
}

void MyDiscordClient::CmdHaipai(Msg msg, const std::vector<std::string> &args)
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
	std::string text;
	for (int i = 0; i < 14; i++) {
		int x = deck.at(i);
		text += std::string(RES, x * 4, 4);
	}
	sendMessage(msg.channelID, text);
}

void MyDiscordClient::SendLargeMessage(
	const std::string &chid, const std::vector<std::string> &lines)
{
	const size_t MsgLenMax = 2000;

	std::string buf;
	buf.reserve(MsgLenMax);
	for (const auto &src : lines) {
		std::string line = src;
		// (バッファ + 改行 + 次の1行) が最大文字数を超えるなら
		// 送信してバッファを空にする
		if (buf.size() + 1 + line.size() > MsgLenMax) {
			if (!buf.empty()) {
				sendMessage(chid, buf);
				buf = "";
			}
		}
		if (buf == "") {
			// 1行で制限を超えるなら超えなくなるまで先に送る
			while (line.size() > MsgLenMax) {
				sendMessage(chid, line.substr(0, MsgLenMax));
				line = line.substr(MsgLenMax);
			}
			// 1行目を追加する
			buf = line;
		}
		else {
			// 2行目以降を追加する
			buf += '\n';
			buf += line;
		}
		if (buf.size() > MsgLenMax) {
			throw std::logic_error("Discord message split logic error");
		}
	}
	if (!buf.empty()) {
		sendMessage(chid, buf);
	}
}

Discord::Discord()
{
	logger.Log(LogLevel::Info, "Initialize Discord...");

	bool enabled = config.GetBool({"Discord", "Enabled"});
	std::string token = config.GetStr({"Discord", "Token"});

	if (enabled) {
		DiscordConfig dconf;
		dconf.DefaultReply = config.GetStr({"Discord", "DefaultReply"});
		dconf.NotifChannel = config.GetStr({"Discord", "NotifChannel"});
		dconf.PrivilegedUsers = config.GetStrArray(
			{"Discord", "PrivilegedUsers"});
		dconf.DenyMessage = config.GetStr({"Discord", "DenyMessage"});

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

void Discord::Send(const std::string &text) noexcept
{
	m_client->SendMessage(text);
}

}	// namespace system
}	// namespace shanghai
