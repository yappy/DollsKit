#ifndef SHANGHAI_TASK_CAMERATASK_H
#define SHANGHAI_TASK_CAMERATASK_H

#include "../taskserver.h"

namespace shanghai {
namespace task {

using namespace std::string_literals;

class CameraTask : public PeriodicTask {
public:
	CameraTask(ReleaseFunc rel_func);
	~CameraTask() = default;

	std::string GetName() override { return "CameraTask"s; }
	void Entry(TaskServer &server, const std::atomic<bool> &cancel) override;

private:
	bool m_enabled;
};

}	// namespace task
}	// namespace shanghai

#endif	// SHANGHAI_TASK_CAMERATASK_H
