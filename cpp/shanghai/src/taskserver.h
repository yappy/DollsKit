#ifndef SHANGHAI_TASKSERVER_H
#define SHANGHAI_TASKSERVER_H

#include <string>
#include <functional>
#include <atomic>
#include <mutex>
#include <condition_variable>
#include <future>
#include <thread>
#include <vector>
#include <queue>

namespace shanghai {

/*
 * 実行終了したサーバの終了理由
 */
enum class ServerResult {
	None,
	Reboot,
	Shutdown,
	ErrorReboot,
	FatalShutdown,
};

class TaskServer;
/*
 * タスクのエントリポイント
 */
using TaskEntry = std::function<void(const std::atomic<bool> &cancel,
	TaskServer &server, const std::string &task_name)>;

/*
 * 1分ごとに確認されリリースされるタスク
 */
struct PeriodicTask {
	std::string name;
	TaskEntry func;
};

/*
 * スレッドプール
 */
class ThreadPool final {
public:
	using TaskFunc = void(const std::atomic<bool> &cancel) noexcept;

	explicit ThreadPool(int thnum);
	~ThreadPool();

	void Shutdown();
	std::future<void> PostTask(TaskFunc func);

private:
	static const int DefaultThreadsNum = 4;

	std::atomic<bool> m_cancel;
	std::mutex m_mtx;
	std::condition_variable m_cond;
	std::vector<std::thread> m_threads;
	std::queue<std::packaged_task<TaskFunc>> m_tasks;
};

/*
 * タスクサーバ本体
 */
class TaskServer final {
public:
	explicit TaskServer(int thnum = std::thread::hardware_concurrency()) :
		m_thread_pool(thnum)
	{}
	~TaskServer() = default;

	ServerResult Run();
	void RequestShutdown();

private:
	std::mutex m_mtx;
	ThreadPool m_thread_pool;
};

}	// namespace shanghai

#endif	// SHANGHAI_LOGGER_H
