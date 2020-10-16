#include "taskconsume.h"
#include "../logger.h"
#include "../system/system.h"

namespace shanghai {
namespace task {

TaskConsumeTask::TaskConsumeTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{}

void TaskConsumeTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	auto &task_queue = system::Get().task_queue;

	system::TaskQueue::TaskFunc func = task_queue.PopFront();
	if (func != nullptr) {
		logger.Log(LogLevel::Trace, "Consume a task");
		func(server, cancel);
	}
}

}	// namespace task
}	// namespace shanghai
