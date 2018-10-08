#include "task.h"
#include "../net.h"

namespace shanghai {
namespace task {

TwitterTask::TwitterTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{
	m_fake_tweet = config.GetBool({"Twitter", "FakeTweet"});
	m_black_list = config.GetStrArray({"Twitter", "BlackList"});
}

void TwitterTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
}

}	// namespace task
}	// namespace shanghai
