#include "house.h"
#include "../util.h"
#include "../logger.h"
#include "../exec.h"

namespace shanghai {
namespace web {

namespace {
	const char * const Timeout = "1";
	const char * const ImgW = "1024";
	const char * const ImgH = "768";
	const char * const ImgTh = "160:120:100";
}	// namespace


HttpResponse HouseTopPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query,
	const PostKeyValueSet &post)
{
	const char *tmpl =
R"(<!DOCTYPE html>

<html lang="en">
<head>
<title>House Management Top Page</title>
</head>

<body>
<h1>House Management Top Page</h1>

<h2>Camera View</h2>
<img src="./pic/take" />

<h2>Switch Control</h2>
<form action="./switch/0" method="POST">
  <input type="submit" value="switch 0">
</form>

</body>
</html>
)";
	return HttpResponse(200, {{"Content-Type", "text/html; charset=utf-8"}},
		tmpl);
}

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

HttpResponse SwitchPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query,
	const PostKeyValueSet &post)
{
	// TODO
	logger.Log(LogLevel::Info, "Switch access: %s", url_match.c_str());

	// 303 See other
	return HttpResponse(303, {{"Location", "/priv/house/"}});
}

}	// namespace web
}	// namespace shanghai
