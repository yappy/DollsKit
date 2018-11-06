#include "task.h"
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
	auto twitter = system::Get().TwitterSystem;
	auto json = twitter.Statuses_HomeTimeline(20);

	for (const auto &entry : json.array_items()) {
		logger.Log(LogLevel::Info, "id=%s screen=%s name=%s",
			entry["id_str"].string_value().c_str(),
			entry["user"]["screen_name"].string_value().c_str(),
			entry["user"]["name"].string_value().c_str());
		logger.Log(LogLevel::Info, "%s", entry["text"].string_value().c_str());
	}
}

}	// namespace task
}	// namespace shanghai
