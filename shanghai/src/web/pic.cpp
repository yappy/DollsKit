#include "pic.h"
#include "../util.h"
#include "../exec.h"

namespace shanghai {
namespace web {

namespace {
	const char * const Timeout = "1";
	const char * const ImgW = "1024";
	const char * const ImgH = "768";
	const char * const ImgTh = "160:120:100";
}	// namespace

HttpResponse PicPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query,
	const PostKeyValueSet &post)
{
	// 写真を stdout に出力する
	Process p("/usr/bin/raspistill",
		{"-o", "-", "-t", Timeout, "-w", ImgW, "-h", ImgH, "-th", ImgTh});
	int exitcode = p.WaitForExit(10);
	if (exitcode != 0) {
		return HttpResponse(500);
	}

	// stdout を image/jpeg として HTTP レスポンスにセット
	return HttpResponse(200,
		{{"Content-Type", "image/jpeg"}},
		p.GetOut());
}

}	// namespace web
}	// namespace shanghai
