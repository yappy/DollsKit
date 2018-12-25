#ifndef SHANGHAI_SYSTEM_TASKQUEUE_H
#define SHANGHAI_SYSTEM_TASKQUEUE_H

#include "../taskserver.h"
#include <mutex>
#include <deque>
#include <functional>

namespace shanghai {
namespace system {

class TaskQueue final {
public:
	using TaskFunc = std::function<
		void(TaskServer &server, const std::atomic<bool> &cancel)>;
	TaskQueue() = default;
	~TaskQueue() = default;

	void Enqueue(TaskFunc &&func);
	TaskFunc PopFront();

private:
	std::mutex m_mtx;
	std::deque<TaskFunc> m_queue;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_TASKQUEUE_H
