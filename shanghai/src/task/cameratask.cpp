#include "task.h"
#include "../system/system.h"

namespace shanghai {
namespace task {

CameraTask::CameraTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{
	m_enabled = config.GetBool({"Camera", "Enabled"});
}

void CameraTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	if (!m_enabled) {
		logger.Log(LogLevel::Info, "[%s] Skipped", GetName().c_str());
		return;
	}

	auto &camera = system::Get().camera;

	std::time_t timestamp = std::time(nullptr);
	std::string id;
	{
		struct tm local;
		char timestr[64] = "";
		::localtime_r(&timestamp, &local);
		int ret = std::strftime(timestr, sizeof(timestr),
			"%Y%m%d_%H%M%S", &local);
		if (ret != 0) {
			id = timestr;
		}
		else {
			id = "unknown";
		}
	}
	logger.Log(LogLevel::Info, "[%s] Take a picture: %s",
		GetName().c_str(), id.c_str());
	camera.Take(id);
}

}	// namespace task
}	// namespace shanghai
