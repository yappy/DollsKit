#include "taskserver.h"
#include "logger.h"

namespace shanghai {

ThreadPool::ThreadPool(int thnum) : m_cancel(false)
{
	if (thnum <= 0) {
		thnum = DefaultThreadsNum;
	}
	// thnum 個のワーカースレッドを立ち上げる
	for (int i = 0; i < thnum; i++) {
		m_threads.emplace_back([this, i]() {
			logger->Log(LogLevel::Info, "Thread pool %d start", i);

			while (1) {
				TaskFunc func;
				{
					// lock
					std::unique_lock<std::mutex> lock(m_mtx);
					// 条件変数待ち
					m_cond.wait(lock, [this]() {
						return m_cancel.load() || !m_tasks.empty();
					});
					// (1) キャンセルが入った場合スレッド終了
					if (m_cancel) {
						break;
					}
					// (2) タスクがキューに存在する場合 pop して処理
					func = std::move(m_tasks.front());
					m_tasks.pop();
					// unlock
				}
				func(m_cancel);
			}

			logger->Log(LogLevel::Info, "Thread pool %d exit", i);
		});
	}
}

ThreadPool::~ThreadPool()
{
	Shutdown();
	for (auto &th : m_threads) {
		th.join();
	}
}

void ThreadPool::Shutdown()
{
	std::unique_lock<std::mutex> lock(m_mtx);
	m_cancel.store(true);
	m_cond.notify_all();
	// unlock
}

void ThreadPool::PostTask(TaskFunc func)
{
	std::unique_lock<std::mutex> lock(m_mtx);
	m_tasks.push(std::move(func));
	m_cond.notify_all();
	// unlock
}

}	// namespace shanghai

