#ifndef SHANGHAI_TASK_TASK_H
#define SHANGHAI_TASK_TASK_H

#include "../logger.h"
#include "../config.h"
#include "../taskserver.h"

namespace shanghai {
namespace task {

using namespace std::string_literals;

class DdnsTask : public PeriodicTask {
public:
	DdnsTask(ReleaseFunc rel_func) : PeriodicTask(rel_func) {}
	~DdnsTask() = default;

	std::string GetName() override
	{
		return "DdnsTask"s;
	}
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override;
};

}	// namespace task
}	// namespace shanghai

#endif	// SHANGHAI_TASK_TASK_H
