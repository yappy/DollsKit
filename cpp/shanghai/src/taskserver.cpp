#include "taskserver.h"

#include <ctime>
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
			logger.Log(LogLevel::Info, "Thread pool %d start", i);

			while (1) {
				std::packaged_task<TaskFunc> task;
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
					task = std::move(m_tasks.front());
					m_tasks.pop();
					// unlock
				}
				task(m_cancel);
			}

			logger.Log(LogLevel::Info, "Thread pool %d exit", i);
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

std::future<void> ThreadPool::PostTask(std::function<TaskFunc> func)
{
	std::unique_lock<std::mutex> lock(m_mtx);
	std::packaged_task<TaskFunc> task(func);
	std::future<void> f = task.get_future();
	m_tasks.push(std::move(task));
	m_cond.notify_all();
	return f;
	// unlock
}


TaskServer::TaskServer(int thnum) :
	m_thread_pool(thnum)
{}

ServerResult TaskServer::Run()
{
	logger.Log(LogLevel::Info, "TaskServer start");
	while (1) {
		std::time_t now = std::time(nullptr);
		struct tm local;
		::localtime_r(&now, &local);
		// ローカル時間から秒を切り捨てて +61 sec したものを次回の起床時刻とする
		local.tm_sec = 0;
		std::time_t target_time = mktime(&local);
		if (target_time == static_cast<std::time_t>(-1)) {
			throw std::system_error(
				std::error_code(errno, std::generic_category()));
		}
		target_time += 60;
		// target_time 以上になるまで待つ
		do {
			now = std::time(nullptr);
			std::time_t sleep_time = (target_time >= now) ?
				target_time - now : 0;
			logger.Log(LogLevel::Trace,
				"sleep for %d sec", static_cast<int>(sleep_time));
			std::this_thread::sleep_for(std::chrono::seconds(sleep_time));
			now = std::time(nullptr);
		} while (now < target_time);

		logger.Log(LogLevel::Trace, "wake up");
	}
	logger.Log(LogLevel::Info, "TaskServer end");
}

void TaskServer::RequestShutdown(ServerResult level)
{
	m_thread_pool.Shutdown();
}

}	// namespace shanghai

