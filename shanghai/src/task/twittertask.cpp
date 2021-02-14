#include "twittertask.h"
#include "../config.h"
#include "../logger.h"
#include "../util.h"
#include "../net.h"
#include "../system/system.h"

namespace shanghai {
namespace task {

TwitterTask::TwitterTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{
	m_black_list = config.GetStrArray({"Twitter", "BlackList"});
	m_black_reply = GetMatchList({"Twitter", "BlackReply"});
	m_replace_list = config.GetStrPairArray({"Twitter", "ReplaceList"});
	m_white_list = config.GetStrArray({"Twitter", "WhiteList"});
	m_white_reply = GetMatchList({"Twitter", "WhiteReply"});
}

void TwitterTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	// 初回実行時のみ
	// 自分の最後のツイート以降でフィルタする
	if (m_since_id == 0) {
		m_since_id = GetInitialSinceId();
		logger.Log(LogLevel::Info, "Initial since_id: %" PRIu64, m_since_id);
	}

	auto &sys_info = system::Get().sys_info;
	auto &twitter = system::Get().twitter;
	// ホームタイムラインを取得
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
		// ID
		uint64_t id = util::to_uint64(status["id_str"].string_value());
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

		std::string white_rep = IsWhite(status);
		std::string black_rep = IsBlack(status);
		if (white_rep != ""s) {
			logger.Log(LogLevel::Info, "Find White");
			log_tweet(status, timestamp);

			sys_info.GetAndSet([](system::SysInfoData &data) {
				data.white++;
			});

			std::string msg = u8"@";
			msg += status["user"]["screen_name"].string_value();
			msg += ' ';
			msg += white_rep;
			twitter.Tweet(msg, status["id_str"].string_value());

			m_since_id = std::max(id, m_since_id);
		}
		else if (black_rep != ""s) {
			logger.Log(LogLevel::Info, "Find Black");
			log_tweet(status, timestamp);

			sys_info.GetAndSet([](system::SysInfoData &data) {
				data.black++;
			});

			std::string msg = u8"@";
			msg += status["user"]["screen_name"].string_value();
			msg += ' ';
			msg += black_rep;
			twitter.Tweet(msg, status["id_str"].string_value());

			m_since_id = std::max(id, m_since_id);
		}
	}
}

TwitterTask::MatchList
TwitterTask::GetMatchList(std::initializer_list<const char *> keys)
{
	const json11::Json &root = config.GetValue(keys);
	if (!root.is_array()) {
		throw ConfigError("Array required: " + Config::CreateKeyName(keys));
	}

	auto string_or_array =
	[&keys](const json11::Json &item) -> std::vector<std::string> {
		std::vector<std::string> result;
		if (item.is_string()) {
			result.emplace_back(item.string_value());
		}
		else if (item.is_array()) {
			for (const auto &elem : item.array_items()) {
				result.emplace_back(elem.string_value());
			}
		}
		else {
			throw ConfigError("String or Array required: " +
				Config::CreateKeyName(keys));
		}
		if (result.size() == 0) {
			throw ConfigError("Array size must be > 0: " +
				Config::CreateKeyName(keys));
		}
		return result;
	};

	MatchList result;
	for (const auto &item : root.array_items()) {
		if (item[0].is_null() || item[1].is_null()) {
			throw ConfigError("Array[2] required: " +
				Config::CreateKeyName(keys));
		}
		result.emplace_back(string_or_array(item[0]), string_or_array(item[1]));
	}

	return result;
}

// 自分のタイムラインの最新 ID を取得する
uint64_t TwitterTask::GetInitialSinceId()
{
	uint64_t since_id = 0;
	auto &twitter = system::Get().twitter;
	auto json = twitter.Statuses_UserTimeline();
	for (const auto &status : json.array_items()) {
		uint64_t id = util::to_uint64(status["id_str"].string_value());
		since_id = std::max(since_id, id);
	}
	return since_id;
}

std::string TwitterTask::Match(const json11::Json &status,
	const std::vector<std::string> &user_filter,
	const MatchList &match_list)
{
	// white/black list filter
	auto in_list = [&status](const std::string &elem) {
		return status["user"]["screen_name"].string_value() == elem;
	};
	if (std::find_if(user_filter.begin(), user_filter.end(), in_list) ==
		user_filter.end()) {
		return "";
	}

	// replace words
	std::string replaced_text = status["text"].string_value();
	for (const auto &pair : m_replace_list) {
		const std::string &from = pair.first;
		const std::string &to = pair.second;
		replaced_text = util::ReplaceAll(replaced_text, from, to);
	}

	// keyword search (AND)
	auto match_word = [&replaced_text](const MatchElem &elem) {
		for (const std::string &word : elem.first) {
			if (replaced_text.find(word) == std::string::npos) {
				return false;
			}
		}
		return true;
	};
	const auto &result = std::find_if(
		match_list.begin(), match_list.end(), match_word);
	if (result != match_list.end()) {
		const auto &list = result->second;
		if (list.empty()) {
			return ""s;
		}
		std::uniform_int_distribution<size_t> dist(0, list.size() - 1);
		size_t random_ind = dist(m_mt);
		return list.at(random_ind);
	}
	else {
		return ""s;
	}
}

// 最先端のヒューリスティクスによるブラック判定
std::string TwitterTask::IsBlack(const json11::Json &status)
{
	return Match(status, m_black_list, m_black_reply);
}

// 最先端のヒューリスティクスによるホワイト判定
std::string TwitterTask::IsWhite(const json11::Json &status)
{
	return Match(status, m_white_list, m_white_reply);
}


RandomTweetTask::RandomTweetTask(ReleaseFunc rel_func) :
	PeriodicTask(rel_func), m_mt(std::random_device()())
{
	m_random_list = config.GetStrArray({"Twitter", "RandomList"});
}

void RandomTweetTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	auto &twitter = system::Get().twitter;
	if (m_random_list.empty()) {
		return;
	}
	std::uniform_int_distribution<size_t> dist(0, m_random_list.size() - 1);
	size_t random_ind = dist(m_mt);
	twitter.Tweet(m_random_list.at(random_ind));
}

}	// namespace task
}	// namespace shanghai
