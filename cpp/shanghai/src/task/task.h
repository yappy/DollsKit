#ifndef SHANGHAI_TASK_TASK_H
#define SHANGHAI_TASK_TASK_H

#include "../logger.h"
#include "../config.h"
#include "../taskserver.h"

namespace shanghai {
// 一定周期でリリースされるタスク群
namespace task {

using namespace std::string_literals;

class HealthCheckTask : public PeriodicTask {
public:
	HealthCheckTask(ReleaseFunc rel_func);
	~HealthCheckTask() = default;

	std::string GetName() override { return "Health"s; }
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override;
};

class DdnsTask : public PeriodicTask {
public:
	DdnsTask(ReleaseFunc rel_func);
	~DdnsTask() = default;

	std::string GetName() override { return "Ddns"s; }
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override;

private:
	bool m_enabled;
	std::string m_user;
	std::string m_pass;
};

class TwitterTask : public PeriodicTask {
public:
	TwitterTask(ReleaseFunc rel_func);
	~TwitterTask() = default;

	std::string GetName() override { return "Twitter"s; }
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override;

private:
	std::vector<std::string> m_black_list;
};

}	// namespace task
}	// namespace shanghai

#endif	// SHANGHAI_TASK_TASK_H
