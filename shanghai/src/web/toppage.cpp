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
  <li>Git branch: {0}</li>
  <li>Git hash: {1}</li>
  <li>White: {2}</li>
  <li>Black: {3}</li>
</ul>

</body>
</html>
)";
	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {
			data.git_branch, data.git_hash,
			std::to_string(data.white), std::to_string(data.black)
		}));
}

}	// namespace web
}	// namespace shanghai
