#include "camera.h"
#include "../logger.h"
#include "../config.h"
#include "../util.h"
#include "../exec.h"

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
	Process p("/bin/mkdir", {"-p", m_picdir});
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

std::string Take()
{
	throw std::logic_error("not implemented");
}

void RemoveOldFiles()
{
	throw std::logic_error("not implemented");
}

}	// namespace system
}	// namespace shanghai
