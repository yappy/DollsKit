#ifndef SHANGHAI_SYSTEM_CAMERA_H
#define SHANGHAI_SYSTEM_CAMERA_H

#include <mutex>
#include <string>

namespace shanghai {
namespace system {

class Camera final {
public:
	Camera();
	~Camera() = default;

	std::string Take();
	void RemoveOldFiles();

private:
	// 周期タスクと web からアクセスされるので排他する
	std::mutex m_mtx;
	std::string m_picdir;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_CAMERA_H
