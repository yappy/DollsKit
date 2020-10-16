#ifndef SHANGHAI_TASK_HEALTHCHECK_H
#define SHANGHAI_TASK_HEALTHCHECK_H

#include "../taskserver.h"

namespace shanghai {
namespace task {

using namespace std::string_literals;

class HealthCheckTask : public PeriodicTask {
public:
	HealthCheckTask(ReleaseFunc rel_func);
	~HealthCheckTask() = default;

	std::string GetName() override { return "Health"s; }
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override;
};

}	// namespace task
}	// namespace shanghai

#endif	// SHANGHAI_TASK_HEALTHCHECK_H
