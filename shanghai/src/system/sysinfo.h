#ifndef SHANGHAI_SYSTEM_SYSINFO_H
#define SHANGHAI_SYSTEM_SYSINFO_H

#include "../taskserver.h"
#include <mutex>

namespace shanghai {
namespace system {

struct SysInfoData {
public:
	std::time_t start_time;
	std::string build_type, git_branch, git_hash, git_date;
	uint32_t task_total, task_comp, task_suc, task_fail;
	uint32_t white, black;
};

class SysInfo final {
public:
	SysInfo() = default;
	~SysInfo() = default;

	SysInfoData Get()
	{
		std::lock_guard<std::mutex> lock(m_mtx);
		return m_data;
	}
	template <class F>
	void GetAndSet(F f)
	{
		std::lock_guard<std::mutex> lock(m_mtx);
		f(std::ref(m_data));
	}

private:
	SysInfoData m_data;
	std::mutex m_mtx;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_SYSINFO_H
