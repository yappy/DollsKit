#include "task.h"
#include "../net.h"

namespace shanghai {
namespace task {

DdnsTask::DdnsTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{
	m_enabled = config.GetBool({"Ddns", "Enabled"});
	m_user = config.GetStr({"Ddns", "User"});
	m_pass = config.GetStr({"Ddns", "Pass"});
}

void DdnsTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	if (!m_enabled) {
		logger.Log(LogLevel::Info, "[%s] Skipped", GetName().c_str());
		return;
	}

	net.DownloadBasicAuth("https://www.mydns.jp/login.html"s,
		m_user, m_pass, 10, cancel);
}

}	// namespace task
}	// namespace shanghai
