#include "webpage.h"
#include "../util.h"

namespace shanghai {
namespace web {

namespace {

HttpResponse PrintJson(const json11::Json &json)
{
	const char *tmpl =
R"(<!DOCTYPE html>
<html lang="en">
<head>
<title>Github hook</title>
</head>
<body>
<code>{0}</code>
</body>
</html>
)";
	std::string json_str = json.is_null() ? "NO DATA" : json.dump();
	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {HtmlEscape(json_str)}));
}

HttpResponse ProcessPost(const std::string &json_str, json11::Json &result)
{
	const char *tmpl =
R"(<!DOCTYPE html>
<html lang="en">
<head>
<title>Github hook</title>
</head>
<body>
{0}
</body>
</html>
)";

	std::string err;
	result = json11::Json::parse(json_str, err);
	if (!err.empty()) {
		// Bad Request
		return HttpResponse(400,
			{{"Content-Type", "text/html; charset=utf-8"}},
			util::Format(tmpl, {HtmlEscape(err)}));
	}

	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {"OK"}));
}

}

HttpResponse GithubPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query,
	const PostKeyValueSet &post)
{
	if (method == "GET") {
		json11::Json json;
		{
			mtx_guard lock(m_mtx);
			json = m_last_push;
		}
		return PrintJson(m_last_push);
	}
	else if (method == "POST") {
		const auto &payload = post.find("payload");
		if (payload == post.end()) {
			// Bad Request
			return HttpResponse(400);
		}
		json11::Json json;
		HttpResponse resp = ProcessPost(payload->second.DataInMemory, json);
		{
			mtx_guard lock(m_mtx);
			m_last_push = json;
		}
		return resp;
	}
	else {
		return HttpResponse(500);
	}
}

}	// namespace web
}	// namespace shanghai
