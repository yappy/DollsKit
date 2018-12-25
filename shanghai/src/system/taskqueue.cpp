#include "taskqueue.h"

namespace shanghai {
namespace system {

namespace {
	using mtx_guard = std::lock_guard<std::mutex>;
}	// namespace

void TaskQueue::Enqueue(TaskFunc &&func)
{
	mtx_guard lock(m_mtx);
	m_queue.emplace_back(func);
}

TaskQueue::TaskFunc TaskQueue::PopFront()
{
	mtx_guard lock(m_mtx);
	if (!m_queue.empty()) {
		auto top = std::move(m_queue.front());
		m_queue.pop_front();
		return top;
	}
	else {
		return nullptr;
	}
}

}	// namespace system
}	// namespace shanghai
