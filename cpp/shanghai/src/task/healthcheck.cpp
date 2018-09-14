#include "task.h"
#include "../net.h"

// Sample:
// [Health Check] CPU Temp: 48.9 cpu:0.5%
// Mem: 581.8/875.7M Avail (66.4%) Disk: 23.2/29.1G Free (79.0%)

namespace shanghai {
namespace task {

HealthCheckTask::HealthCheckTask(ReleaseFunc rel_func) : PeriodicTask(rel_func)
{
}

void HealthCheckTask::Entry(TaskServer &server, const std::atomic<bool> &cancel)
{

}

}	// namespace task
}	// namespace shanghai
