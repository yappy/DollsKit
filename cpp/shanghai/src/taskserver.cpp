#include "taskserver.h"
#include "logger.h"

namespace shanghai {

ThreadPool::ThreadPool(int thnum)
{
	if (thnum <= 0) {
		thnum = DefaultThreadsNum;
	}
	for (int i = 0; i < thnum; i++) {
		m_threads.emplace_back([i](){
			logger->Log(LogLevel::Info, "Thread %d start", i);
		});
	}
}

ThreadPool::~ThreadPool()
{
	for (auto &th : m_threads) {
		th.join();
	}
}

}	// namespace shanghai

