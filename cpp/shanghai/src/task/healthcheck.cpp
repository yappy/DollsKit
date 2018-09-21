#include "task.h"
#include "../util.h"
#include <chrono>
#include <thread>

// Sample:
// [Health Check] CPU Temp: 48.9 cpu:0.5%
// Mem: 581.8/875.7M Avail (66.4%) Disk: 23.2/29.1G Free (79.0%)

namespace shanghai {
namespace task {

namespace {

using namespace std::chrono_literals;

// CPU 使用率の測定時間
const int CpuMeasureSec = 5;

std::string GetCpuUsage(const std::atomic<bool> &cancel)
{
	// 0.name
	// 1.user  2.nice  3.system  4.idle  5.iowait
	// 6.irq  7.softirq  8.steal  9.guest  10.guest_nice
	// (後ろの方は kernel version による)

	struct CpuTime {
		// [1]..[end] の合計
		uint64_t total;
		// [4]
		uint64_t idle;
	};
	struct CpuStat {
		// [0] : "cpu"
		CpuTime total;
		// [0] : "cpu%d"
		std::vector<CpuTime> cpus;
	};
	auto read_func = []() -> CpuStat {
		CpuStat result;
		std::string all = util::ReadStringFromFile("/proc/stat"s);
		for (const std::string &line : util::Split(all, '\n', true)) {
			std::vector<std::string> elems = util::Split(line, ' ', true);
			// string::starts_with は C++20
			// 速度はいらないので find() == 0 でしのぐ
			if (elems.size() > 0 && elems.at(0).find("cpu") == 0) {
				CpuTime cpu_time = { 0, 0 };
				for (decltype(elems.size()) i = 1; i < elems.size(); i++) {
					uint64_t jiff = std::stoull(elems.at(i));
					cpu_time.total += jiff;
					if (i == 4) {
						cpu_time.idle = jiff;
					}
				}
				if (elems.at(0) == "cpu") {
					result.total = cpu_time;
				}
				else {
					result.cpus.emplace_back(cpu_time);
				}
			}
		}
		return result;
	};

	CpuStat stat1 = read_func();
	for (int i = 0; i < CpuMeasureSec; i++) {
		if (cancel.load()) {
			throw CancelError("Cancel in CPU measurement");
		}
		std::this_thread::sleep_for(1s);
	}
	CpuStat stat2 = read_func();

	std::string result;
	double total = 1.0 -
		(double)(stat2.total.idle - stat1.total.idle) /
		(double)(stat2.total.total - stat1.total.total);
	result += "CPU: ";
	result += util::ToString("%.1f", total * 100);
	result += '%';

	auto cpunum = std::min(stat1.cpus.size(), stat2.cpus.size());
	if (cpunum > 0) {
		result += " (";
		bool is_first = true;
		for (decltype(cpunum) i = 0; i < cpunum; i++) {
			if (is_first) {
				is_first = false;
			}
			else {
				result += ' ';
			}
			double cpu = 1.0 -
				(double)(stat2.cpus.at(i).idle - stat1.cpus.at(i).idle) /
				(double)(stat2.cpus.at(i).total - stat1.cpus.at(i).total);
			result += util::ToString("%.1f", cpu * 100);
		}
		result += ")";
	}

	return result;
}

}	// namespace

HealthCheckTask::HealthCheckTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{

}

void HealthCheckTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{
	logger.Log(LogLevel::Info, "%s", GetCpuUsage(cancel).c_str());
}

}	// namespace task
}	// namespace shanghai
