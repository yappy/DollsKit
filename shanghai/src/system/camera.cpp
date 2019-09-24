#include "camera.h"
#include "../logger.h"
#include "../config.h"

namespace shanghai {
namespace system {

Camera::Camera()
{
	logger.Log(LogLevel::Info, "Initialize Camera...");

	logger.Log(LogLevel::Info, "Initialize Camera OK");
}

}	// namespace system
}	// namespace shanghai
