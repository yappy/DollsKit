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

	Count
};

const std::array<const char *, static_cast<int>(ServerResult::Count)>
ServerResultStr = {
	"None",
	"Reboot",
	"Shutdown",
	"ErrorReboot",
	"FatalShutdown",
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
	using TaskFunc = void(const std::atomic<bool> &cancel);

	explicit ThreadPool(int thnum);
	~ThreadPool();

	bool Shutdown(int timeout_sec);
	std::future<void> PostTask(std::function<TaskFunc> func);

private:
	static const int DefaultThreadsNum = 4;

	std::mutex m_mtx;
	std::condition_variable m_task_cond, m_exit_cond;
	std::atomic<bool> m_cancel;
	int m_active_count;
	std::vector<std::thread> m_threads;
	std::queue<std::packaged_task<TaskFunc>> m_tasks;
};

/*
 * タスクサーバ本体
 */
class TaskServer final {
public:
	explicit TaskServer(int thnum = std::thread::hardware_concurrency());
	~TaskServer() = default;

	ServerResult Run();
	void RequestShutdown(ServerResult result);

private:
	static const int ShutdownTimeout = 60;

	std::mutex m_mtx;
	std::condition_variable m_shutdown_cond;
	ThreadPool m_thread_pool;
	ServerResult m_result;
};

}	// namespace shanghai

#endif	// SHANGHAI_TASKSERVER_H
