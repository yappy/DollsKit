#include "camera.h"
#include "../logger.h"
#include "../config.h"
#include "../util.h"
#include "../exec.h"

using mtx_guard = std::lock_guard<std::mutex>;

namespace shanghai {
namespace system {

Camera::Camera()
{
	logger.Log(LogLevel::Info, "Initialize Camera...");

	m_picdir = config.GetStr({"Camera", "PicDir"});
	logger.Log(LogLevel::Info, "Picture dir: %s", m_picdir.c_str());
	// std::filesystem が gcc 6.3.0 ではまだ experimental であり、
	// コンパイラバージョン依存のソースとリンク設定(cmake)を書くのが煩雑なので
	// mkdir -p を呼ぶ
	Process p{"/bin/mkdir", {"-p", m_picdir}};
	int exitcode = p.WaitForExit();
	if (exitcode != 0) {
		throw FileError(p.GetErr());
	}

	std::vector<std::string> files = util::EnumFiles(m_picdir + "/*.jpg");
	logger.Log(LogLevel::Info, "Camera picture: %zu files", files.size());
	for (const auto &file : files) {
		logger.Log(LogLevel::Trace, "%s", file.c_str());
	}

	logger.Log(LogLevel::Info, "Initialize Camera OK");
}

void Camera::Take(const std::string &path,
	uint32_t timeout_ms,
	uint32_t w, uint32_t h,
	uint32_t th_w, uint32_t th_h,
	uint32_t th_quality)
{
	mtx_guard lock{m_mtx};

	// raspistill -o <path> -w <wsize> -h <hsize> -th <w>:<h>:<quality>
	Process p{"/usr/bin/raspistill", {
		"-o", path,
		"-w", std::to_string(w),
		"-h", std::to_string(h),
		"-th", util::Format("{0}:{1}:{2}", {
			std::to_string(th_w), std::to_string(th_h),
			std::to_string(th_quality)
		}),
	}};
	p.WaitForExit();
}

void Camera::RemoveOldFiles()
{
	throw std::logic_error("not implemented");
}

}	// namespace system
}	// namespace shanghai
