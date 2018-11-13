#include "task.h"
#include "../util.h"
#include "../net.h"
#include "../system/system.h"

namespace shanghai {
namespace task {

TwitterTask::TwitterTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{
	m_black_list = config.GetStrArray({"Twitter", "BlackList"});
	m_black_words = config.GetStrArray({"Twitter", "BlackWords"});
	m_replace_list = config.GetStrPairArray({"Twitter", "ReplaceList"});
	m_white_list = config.GetStrArray({"Twitter", "WhiteList"});
	m_white_words = config.GetStrArray({"Twitter", "WhiteWords"});
}

void TwitterTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	// 初回実行時のみ
	// 自分の最後のツイート以降でフィルタする
	if (m_since_id == 0) {
		m_since_id = GetInitialSinceId();
		logger.Log(LogLevel::Info, "Initial since_id: %" PRIu64, m_since_id);
	}

	// ホームタイムラインを取得
	auto twitter = system::Get().twitter;
	auto json = twitter.Statuses_HomeTimeline({
		{"since_id", std::to_string(m_since_id)},
		{"count", "200"}});

	auto log_tweet = [](const json11::Json &status, std::time_t timestamp) {
		logger.Log(LogLevel::Info, "id=%s time=%s local=%s screen=%s name=%s",
			status["id_str"].string_value().c_str(),
			status["created_at"].string_value().c_str(),
			util::DateTimeStr(timestamp).c_str(),
			status["user"]["screen_name"].string_value().c_str(),
			status["user"]["name"].string_value().c_str());
		logger.Log(LogLevel::Info, "%s", status["text"].string_value().c_str());
	};

	for (const auto &status : json.array_items()) {
		// ローカルタイムに変換
		std::time_t timestamp = util::StrToTimeTwitter(
			status["created_at"].string_value());
		struct tm local;
		::localtime_r(&timestamp, &local);
		// 自分のツイートには反応しない
		if (util::to_uint64(status["id_str"].string_value()) == twitter.MyId()) {
			continue;
		}
		// リツイートには反応しない
		if (!status["retweeted_status"].is_null()) {
			continue;
		}
		if (IsWhite(status)) {
			logger.Log(LogLevel::Info, "Find White");
			log_tweet(status, timestamp);

			std::string msg = u8"@";
			msg += status["user"]["screen_name"].string_value();
			msg += ' ';
			msg += u8"ホワイト！";
			twitter.Tweet(msg, status["id_str"].string_value());
		}
		if (IsBlack(status)) {
			logger.Log(LogLevel::Info, "Find Black");
			log_tweet(status, timestamp);

			std::string msg = u8"@";
			msg += status["user"]["screen_name"].string_value();
			msg += ' ';
			msg += u8"ブラック";
			twitter.Tweet(msg, status["id_str"].string_value());
		}
	}
}

// 自分のタイムラインの最新 ID を取得する
uint64_t TwitterTask::GetInitialSinceId()
{
	uint64_t since_id = 0;
	auto twitter = system::Get().twitter;
	auto json = twitter.Statuses_UserTimeline();
	for (const auto &entry : json.array_items()) {
		uint64_t id = util::to_uint64(entry["id_str"].string_value());
		since_id = std::max(since_id, id);
	}
	return since_id;
}

// 最先端のヒューリスティクスによるブラック判定
bool TwitterTask::IsBlack(const json11::Json &status)
{
	// black list filter
	auto in_list = [&status](const std::string elem) {
		return status["user"]["screen_name"].string_value() == elem;
	};
	if (std::find_if(m_black_list.begin(), m_black_list.end(), in_list) ==
		m_black_list.end()) {
		return false;
	}

	// replace words
	std::string replaced_text = status["text"].string_value();
	for (const auto &pair : m_replace_list) {
		const std::string &from = pair.first;
		const std::string &to = pair.second;
		replaced_text = util::ReplaceAll(replaced_text, from, to);
	}

	// keyword search
	auto match_word = [&replaced_text](const std::string elem) {
		return replaced_text.find(elem) != std::string::npos;
	};
	if (std::find_if(m_black_words.begin(), m_black_words.end(), match_word) !=
		m_black_words.end()) {
		return true;
	}
	return false;
}

bool TwitterTask::IsWhite(const json11::Json &status)
{
	// white list filter
	auto in_list = [&status](const std::string elem) {
		return status["user"]["screen_name"].string_value() == elem;
	};
	if (std::find_if(m_white_list.begin(), m_white_list.end(), in_list) ==
		m_white_list.end()) {
		return false;
	}

	// keyword search
	auto match_word = [&status](const std::string elem) {
		return status["text"].string_value().find(elem) != std::string::npos;
	};
	if (std::find_if(m_white_words.begin(), m_white_words.end(), match_word) !=
		m_white_words.end()) {
		return true;
	}
	return false;
}

}	// namespace task
}	// namespace shanghai
