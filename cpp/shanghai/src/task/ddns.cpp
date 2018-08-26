#include "task.h"
#include "../net.h"

namespace shanghai {
namespace task {

void DdnsTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	net.Download("http://www.mydns.jp/login.html"s, 10, cancel);

}

}	// namespace task
}	// namespace shanghai
