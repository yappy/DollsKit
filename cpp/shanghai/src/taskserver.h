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
 * 1分ごとに確認されリリースされるタスク
 */
class PeriodicTask {
public:
	using ReleaseFunc = std::function<bool(const struct tm &local_time)>;

	explicit PeriodicTask(ReleaseFunc rel_func) : m_rel_func(rel_func) {}
	virtual ~PeriodicTask() = default;

	virtual std::string GetName() = 0;
	bool CheckRelease(const struct tm &local_time)
	{
		return m_rel_func(local_time);
	}
	virtual void Entry(TaskServer &server, const std::atomic<bool> &cancel) = 0;

private:
	ReleaseFunc m_rel_func;
};

class OneShotTask {
public:
	using TaskFunc = std::function<
		void(TaskServer &server, const std::atomic<bool> &cancel)>;

	OneShotTask(std::string name, TaskFunc func) : m_name(name), m_func(func)
	{}
	~OneShotTask() = default;

	const std::string &GetName() { return m_name; }
	void Entry(TaskServer &server, const std::atomic<bool> &cancel)
	{
		m_func(server, cancel);
	}

private:
	std::string m_name;
	TaskFunc m_func;
};

/*
 * スレッドプール
 */
class ThreadPool final {
public:
	using TaskFunc = void(const std::atomic<bool> &cancel) noexcept;

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

	void RegisterPeriodicTask(std::unique_ptr<PeriodicTask> &&task);
	void RegisterOneShotTask(OneShotTask &&task);
	void ReleaseAllForTest();

	ServerResult Run();
	void RequestShutdown(ServerResult result);

private:
	static const int ShutdownTimeout = 60;

	std::mutex m_mtx;
	std::condition_variable m_shutdown_cond;
	ThreadPool m_thread_pool;

	bool m_started;
	ServerResult m_result;
	std::vector<std::unique_ptr<PeriodicTask>> m_periodic_list;

	std::future<void> ReleaseTask(const std::unique_ptr<PeriodicTask> &task);
};

}	// namespace shanghai

#endif	// SHANGHAI_TASKSERVER_H
