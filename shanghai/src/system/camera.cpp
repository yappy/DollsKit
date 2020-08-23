#include "camera.h"
#include "../logger.h"
#include "../config.h"
#include "../util.h"
#include "../exec.h"
#include <algorithm>
#include <filesystem>

/*
https://www.raspberrypi.org/documentation/raspbian/applications/camera.md
*/

namespace fs = std::filesystem;
using mtx_guard = std::lock_guard<std::mutex>;

namespace shanghai {
namespace system {

Camera::Camera()
{
	logger.Log(LogLevel::Info, "Initialize Camera...");

	m_picdir = config.GetStr({"Camera", "PicDir"});
	logger.Log(LogLevel::Info, "Picture dir: %s", m_picdir.c_str());
	bool mkdir = fs::create_directories(m_picdir);
	logger.Log(LogLevel::Info, "Created: %s", mkdir ? "Yes" : "No");

	logger.Log(LogLevel::Info, "Initialize Camera OK");
}

void Camera::Take(const std::string &path, bool abspath,
	std::string *stdout,
	uint32_t timeout_ms,
	uint32_t w, uint32_t h,
	uint32_t th_w, uint32_t th_h,
	uint32_t th_quality)
{
	mtx_guard lock{m_mtx};

	fs::path outpath;
	if (!abspath) {
		outpath /= m_picdir;
	}
	outpath /= path;

	// raspistill -o <path> -w <wsize> -h <hsize> -th <w>:<h>:<quality>
	Process p {"/usr/bin/raspistill", {
		"-o", outpath.string(),
		"-t", std::to_string(timeout_ms),
		"-w", std::to_string(w),
		"-h", std::to_string(h),
		"-th", util::Format("{0}:{1}:{2}", {
			std::to_string(th_w), std::to_string(th_h),
			std::to_string(th_quality)
		}),
	}};
	// 5秒余計に待ってみる (タイムアウトは例外発生)
	int exitcode = p.WaitForExit(5 + timeout_ms / 1000);
	if (exitcode != 0) {
		logger.Log(LogLevel::Error, "raspistill: %d", exitcode);
		throw ProcessError("raspistill exit code is not 0");
	}

	if (stdout != nullptr) {
		p.GetOut().swap(*stdout);
	}
}

std::vector<std::string> Camera::GetFileList()
{
	std::vector<std::string> files;
	for (const auto &entry : fs::directory_iterator(m_picdir)) {
		if (fs::is_regular_file(entry.path())) {
			files.emplace_back(entry.path().u8string());
		}
	}
	std::sort(files.begin(), files.end());
	return files;
}

void Camera::RemoveOldFiles()
{
	throw std::logic_error("not implemented");
}

}	// namespace system
}	// namespace shanghai
