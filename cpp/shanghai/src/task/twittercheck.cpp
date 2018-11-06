#include "task.h"
#include "../util.h"
#include "../net.h"
#include "../system/system.h"

namespace shanghai {
namespace task {

TwitterTask::TwitterTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{
	m_black_list = config.GetStrArray({"Twitter", "BlackList"});
}

void TwitterTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	// 初回実行時のみ
	if (m_since_id == 0) {
		m_since_id = GetInitialSinceId();
		logger.Log(LogLevel::Info, "Initial since_id: %" PRIu64, m_since_id);
	}

	auto twitter = system::Get().TwitterSystem;
	auto json = twitter.Statuses_HomeTimeline();

	for (const auto &entry : json.array_items()) {
		logger.Log(LogLevel::Info, "id=%s screen=%s name=%s",
			entry["id_str"].string_value().c_str(),
			entry["user"]["screen_name"].string_value().c_str(),
			entry["user"]["name"].string_value().c_str());
		logger.Log(LogLevel::Info, "%s", entry["text"].string_value().c_str());
	}
}

// 自分のタイムラインの最新 ID を取得する
uint64_t TwitterTask::GetInitialSinceId()
{
	uint64_t since_id = 0;
	auto twitter = system::Get().TwitterSystem;
	auto json = twitter.Statuses_UserTimeline();
	for (const auto &entry : json.array_items()) {
		uint64_t id = util::to_uint64(entry["id_str"].string_value());
		since_id = std::max(since_id, id);
	}
	return since_id;
}

}	// namespace task
}	// namespace shanghai
