#ifndef SHANGHAI_SYSTEM_CAMERA_H
#define SHANGHAI_SYSTEM_CAMERA_H

#include <string>

namespace shanghai {
namespace system {

class Camera final {
public:
	Camera();
	~Camera() = default;

	std::string Take();

private:
	std::string m_picdir;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_CAMERA_H
