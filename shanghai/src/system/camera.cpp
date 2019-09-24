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
	// std::filesystem が gcc 6.3.0 ではまだ experimental であり、
	// コンパイラバージョン依存のソースとリンク設定(cmake)を書くのが煩雑なので
	// mkdir -p を呼ぶ
	Process p("/bin/mkdir", {"-p", m_picdir});
	int exitcode = p.WaitForExit();
	if (exitcode != 0) {
		throw FileError(p.GetErr());
	}

	logger.Log(LogLevel::Info, "Initialize Camera OK");
}

}	// namespace system
}	// namespace shanghai
