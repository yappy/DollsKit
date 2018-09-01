#include "task.h"
#include "../net.h"

namespace shanghai {
namespace task {

void DdnsTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	net.DownloadBasicAuth("https://www.mydns.jp/login.html"s,
		"user", "pass", 10, cancel);

}

}	// namespace task
}	// namespace shanghai
