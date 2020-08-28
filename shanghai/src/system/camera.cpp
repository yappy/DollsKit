#include "camera.h"
#include "../logger.h"
#include "../config.h"
#include "../util.h"
#include "../exec.h"
#include <algorithm>
#include <regex>
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

void Camera::Take(const std::string &id,
	uint32_t timeout_ms,
	uint32_t w, uint32_t h,
	uint32_t th_w, uint32_t th_h,
	uint32_t th_quality)
{
	mtx_guard lock{m_mtx};

	fs::path outpath, thpath;
	outpath /= m_picdir;
	outpath /= util::Format("{0}.jpg", {id});
	thpath /= m_picdir;
	thpath /= util::Format("{0}_th.jpg", {id});

	TakeInternal(outpath, timeout_ms, w, h, th_w, th_h, th_quality);

	// Exif 情報を削除して上書き
	{
		Process p {"/usr/bin/convert", {
			outpath, "-thumbnail", "100%", outpath
		}};
		int exitcode = p.WaitForExit();
		if (exitcode != 0) {
			logger.Log(LogLevel::Error, "raspistill: %d", exitcode);
			throw ProcessError("raspistill exit code is not 0");
		}
	}
	// サムネイルを作る
	{
		Process p {"/usr/bin/convert", {
			outpath, "-thumbnail", "160x", thpath
		}};
		int exitcode = p.WaitForExit();
		if (exitcode != 0) {
			logger.Log(LogLevel::Error, "convert: %d", exitcode);
			throw ProcessError("convert exit code is not 0");
		}
	}
}

std::string Camera::TakeToStdout(
	uint32_t timeout_ms,
	uint32_t w, uint32_t h,
	uint32_t th_w, uint32_t th_h,
	uint32_t th_quality)
{
	mtx_guard lock{m_mtx};
	return TakeInternal("-", timeout_ms, w, h, th_w, th_h, th_quality);
}

std::string Camera::TakeInternal(
	const std::string &path,
	uint32_t timeout_ms,
	uint32_t w, uint32_t h, uint32_t th_w, uint32_t th_h,
	uint32_t th_quality)
{
	// raspistill -o <path> -w <wsize> -h <hsize> -th <w>:<h>:<quality>
	Process p {"/usr/bin/raspistill", {
		"-o", path,
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
	std::string stdout;
	p.GetOut().swap(stdout);
	return stdout;
}

std::vector<Camera::PicEntry> Camera::GetFileList()
{
	mtx_guard lock{m_mtx};

	// (/path/to/dir/(id))_th.jpg
	std::regex th_re(R"((.*/(\w+))_th\.jpg)");
	std::smatch match;

	std::vector<PicEntry> files;
	for (const auto &entry : fs::directory_iterator(m_picdir)) {
		const auto &path = entry.path();
		const auto &pathstr = path.u8string();
		if (fs::is_regular_file(path)) {
			if (std::regex_match(pathstr, match, th_re)) {
				files.emplace_back(
					match[2],
					util::Format("{0}.jpg", {match[1]}),
					pathstr);
			}
		}
	}
	std::sort(files.begin(), files.end());
	return files;
}

void Camera::RemoveOldFiles()
{
	mtx_guard lock{m_mtx};
	throw std::logic_error("not implemented");
}

}	// namespace system
}	// namespace shanghai
