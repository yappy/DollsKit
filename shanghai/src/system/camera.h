/*
 * Command line full documentation
 * https://www.raspberrypi.org/documentation/raspbian/applications/camera.md
 * H/W S/W features
 * https://www.raspberrypi.org/documentation/hardware/camera/
 */

#ifndef SHANGHAI_SYSTEM_CAMERA_H
#define SHANGHAI_SYSTEM_CAMERA_H

#include <mutex>
#include <string>

namespace shanghai {
namespace system {

class Camera final {
public:
	static const uint32_t DEFAULT_TIMEOUT_MS = 5000;
	// Camera Module v2
	static const uint32_t DEFAULT_W = 3280;
	static const uint32_t DEFAULT_H = 2464;
	// thumbnail
	static const uint32_t DEFAULT_THUMB_W = 160;
	static const uint32_t DEFAULT_THUMB_H = 120;
	static const uint32_t DEFAULT_THUMB_QUALITY = 100;

	Camera();
	~Camera() = default;

	void Take(const std::string &path,
		uint32_t timeout_ms = DEFAULT_TIMEOUT_MS,
		uint32_t w = DEFAULT_W, uint32_t h = DEFAULT_H,
		uint32_t th_w = DEFAULT_THUMB_W, uint32_t th_h = DEFAULT_THUMB_H,
		uint32_t th_quality = DEFAULT_THUMB_QUALITY);
	void RemoveOldFiles();

private:
	// 周期タスクと web からアクセスされるので排他する
	std::mutex m_mtx;
	std::string m_picdir;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_CAMERA_H
