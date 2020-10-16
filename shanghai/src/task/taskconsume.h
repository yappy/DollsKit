#ifndef SHANGHAI_TASK_TASKCONSUME_H
#define SHANGHAI_TASK_TASKCONSUME_H

#include "../taskserver.h"

namespace shanghai {
namespace task {

using namespace std::string_literals;

class TaskConsumeTask : public PeriodicTask {
public:
	TaskConsumeTask(ReleaseFunc rel_func);
	~TaskConsumeTask() = default;

	std::string GetName() override { return "TaskQueue"s; }
	bool IsQuiet() override { return true; }
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override;
};


}	// namespace task
}	// namespace shanghai

#endif	// SHANGHAI_TASK_TASKCONSUME_H
