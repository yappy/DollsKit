#include "system.h"
#include "../logger.h"
#include <stdexcept>
#include <memory>

namespace shanghai {
namespace system {

namespace {
	std::unique_ptr<System> s_system;
}	// namespace

void Initialize()
{
	if (s_system != nullptr) {
		throw std::logic_error("System is already initialized");
	}
	logger.Log(LogLevel::Info, "Initialize system...");
	s_system = std::make_unique<System>();
	logger.Log(LogLevel::Info, "Initialize system OK");
}

void Finalize() noexcept
{
	logger.Log(LogLevel::Info, "Finalize system OK");
	s_system.reset();
	logger.Log(LogLevel::Info, "Finalize system OK");
}

System &Get()
{
	return *s_system;
}

}	// namespace system
}	// namespace shanghai
