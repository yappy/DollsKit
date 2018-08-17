#include "taskserver.h"

#include <ctime>
#include "logger.h"

namespace shanghai {

ThreadPool::ThreadPool(int thnum) : m_cancel(false)
{
	if (thnum <= 0) {
		thnum = DefaultThreadsNum;
	}
	m_active_count = thnum;
	// thnum 個のワーカースレッドを立ち上げる
	for (int i = 0; i < thnum; i++) {
		m_threads.emplace_back([this, i]() {
			logger.Log(LogLevel::Info, "Thread pool %d start", i);

			while (1) {
				std::packaged_task<TaskFunc> task;
				{
					std::unique_lock<std::mutex> lock(m_mtx);
					// キャンセル条件変数待ち
					m_task_cond.wait(lock, [this]() {
						return m_cancel.load() || !m_tasks.empty();
					});
					// (1) キャンセルが入った場合スレッド終了
					if (m_cancel) {
						break;
					}
					// (2) タスクがキューに存在する場合先頭から取り出して処理
					task = std::move(m_tasks.front());
					m_tasks.pop();
				}
				// 実行して future に結果をセット (void or exception)
				task(m_cancel);
			}
			logger.Log(LogLevel::Info, "Thread pool %d exit", i);
			{
				// アクティブスレッド数をデクリメントして 0 になったら signal
				std::unique_lock<std::mutex> lock(m_mtx);
				m_active_count--;
				if (m_active_count == 0) {
					m_exit_cond.notify_all();
				}
			}
		});
	}
}

ThreadPool::~ThreadPool()
{
	Shutdown(0);
	for (auto &th : m_threads) {
		th.join();
	}
}

bool ThreadPool::Shutdown(int timeout_sec)
{
	std::unique_lock<std::mutex> lock(m_mtx);
	m_cancel.store(true);
	m_task_cond.notify_all();

	// アクティブスレッド数 0 条件待ち、タイムアウトしたら false
	return m_exit_cond.wait_for(lock, std::chrono::seconds(timeout_sec),
		[this]() -> bool {
			return m_active_count == 0;
		});
}

std::future<void> ThreadPool::PostTask(std::function<TaskFunc> func)
{
	// packaged_task を作ってキューに追加、future をここから返す
	// worker thread に signal
	std::unique_lock<std::mutex> lock(m_mtx);
	std::packaged_task<TaskFunc> task(func);
	std::future<void> f = task.get_future();
	m_tasks.push(std::move(task));
	m_task_cond.notify_all();
	return f;
}


TaskServer::TaskServer(int thnum) :
	m_thread_pool(thnum),
	m_result(ServerResult::None)
{}

void TaskServer::RegisterPeriodicTask(const PeriodicTask &task)
{
	std::lock_guard<std::mutex> lock(m_mtx);
	// copy construct
	m_periodic_list.emplace_back(task);
}

ServerResult TaskServer::Run()
{
	ServerResult result = ServerResult::None;

	logger.Log(LogLevel::Info, "TaskServer start");
	while (1) {
		std::time_t now = std::time(nullptr);
		struct tm local;
		::localtime_r(&now, &local);
		// ローカル時間から秒を切り捨てて +60 sec したものを次回の起床時刻とする
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
			// シャットダウン条件変数をタイムアウト付きで待つ
			{
				std::unique_lock<std::mutex> lock(m_mtx);
				bool shutdown = m_shutdown_cond.wait_for(
					lock, std::chrono::seconds(sleep_time),
					[this]() -> bool {
						return m_result != ServerResult::None;
					});
				// シャットダウン条件で起きた場合
				// その時のロック状態でローカル変数にコピー、それを return
				if (shutdown) {
					result = m_result;
					goto END;
				}
				// unlock
			}
			now = std::time(nullptr);
		} while (now < target_time);

		logger.Log(LogLevel::Trace, "wake up");

		::localtime_r(&now, &local);
		{
			std::lock_guard<std::mutex> lock(m_mtx);
			for (auto &task : m_periodic_list) {
				if (task.cond(local)) {
					std::string name = task.name;
					TaskEntry entry = task.entry;
					m_thread_pool.PostTask(
						[this, entry, name](const std::atomic<bool> &cancel)
						{
							entry(cancel, *this, name);
						});
				}
			}
		}
	}
END:
	// スレッドプールにシャットダウン要求を入れて終了待ち
	bool pool_ok = m_thread_pool.Shutdown(ShutdownTimeout);
	// タイムアウトした: スレッドプールのデストラクタ内 join() で固まると思われるため
	// このインスタンスの破棄前に std::terminate() が必要
	if (!pool_ok) {
		logger.Log(LogLevel::Fatal, "Thread pool shutdown timeout");
		return ServerResult::FatalShutdown;
	}

	logger.Log(LogLevel::Info, "TaskServer end: %s",
		ServerResultStr.at(static_cast<int>(result)));
	return result;
}

void TaskServer::RequestShutdown(ServerResult result)
{
	if (result == ServerResult::None) {
		throw std::runtime_error("Invalid result");
	}
	{
		std::unique_lock<std::mutex> lock(m_mtx);
		m_result = result;
		m_shutdown_cond.notify_all();
	}
}

}	// namespace shanghai

