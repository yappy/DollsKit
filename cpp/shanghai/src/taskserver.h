#ifndef SHANGHAI_TASKSERVER_H
#define SHANGHAI_TASKSERVER_H

#include <string>
#include <functional>
#include <atomic>
#include <mutex>
#include <thread>
#include <vector>

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
 * 1分ごとに確認されリリースされるタスク
 */
struct PeriodicTask {
	std::string name;
	std::function<void(TaskServer &, const std::string)> proc;
};

/*
 * スレッドプール
 */
class ThreadPool final {
public:
	explicit ThreadPool(int thnum);
	~ThreadPool();

	void Shutdown();

private:
	static const int DefaultThreadsNum = 4;

	std::vector<std::thread> m_threads;
};

/*
 * タスクサーバ本体
 */
class TaskServer final {
public:
	explicit TaskServer(int thnum = std::thread::hardware_concurrency()) :
		m_cancel(false),
		m_thread_pool(thnum)
	{}
	~TaskServer() = default;

	ServerResult Run();
	void RequestShutdown();

private:
	std::atomic<bool> m_cancel;
	std::mutex m_mtx;
	ThreadPool m_thread_pool;
};

}	// namespace shanghai

#endif	// SHANGHAI_LOGGER_H
