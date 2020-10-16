#ifndef SHANGHAI_TASK_DDNS_H
#define SHANGHAI_TASK_DDNS_H

#include "../taskserver.h"

namespace shanghai {
namespace task {

using namespace std::string_literals;

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

}	// namespace task
}	// namespace shanghai

#endif	// SHANGHAI_TASK_DDNS_H
