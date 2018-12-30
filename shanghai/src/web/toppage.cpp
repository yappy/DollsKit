#include "toppage.h"
#include "../util.h"
#include "../system/system.h"

namespace shanghai {
namespace web {

HttpResponse TopPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query,
	const PostKeyValueSet &post)
{
	auto &sys_info = system::Get().sys_info;
	system::SysInfoData data = sys_info.Get();

	std::time_t now = std::time(nullptr);
	std::time_t dur = static_cast<int64_t>(now - data.start_time);
	int64_t day = dur / (60 * 60 * 24);
	dur %= (60 * 60 * 24);
	int64_t hour = dur / (60 * 60);
	dur %= (60 * 60);
	int64_t min = dur / 60;
	dur %= 60;
	int64_t sec = dur;
	std::string durstr = util::Format("{0} day, {1} hour, {2} min, {3} sec", {
		std::to_string(day), std::to_string(hour),
		std::to_string(min), std::to_string(sec)});

	const char *tmpl =
R"(<!DOCTYPE html>

<html lang="en">
<head>
<title>System Available</title>
</head>

<body>
<h1>System Available</h1>

<h2>Summary</h2>
<ul>
  <li>Started: {0}</li>
  <li>Operating time: {1}</li>
  <li>Git branch: {2}</li>
  <li>Git hash: {3}</li>
  <li>White: {4}</li>
  <li>Black: {5}</li>
</ul>

</body>
</html>
)";
	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {
			util::DateTimeStr(data.start_time), durstr,
			data.git_branch, data.git_hash,
			std::to_string(data.white), std::to_string(data.black)
		}));
}

}	// namespace web
}	// namespace shanghai
