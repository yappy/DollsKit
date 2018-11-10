/*
 * Github repository -> Settings -> Webhhoks
 * Content type: application/json を指定のこと
 * application/x-www-form-urlencoded では libmicrohttpd の POST プロセッサが
 * パースしてしまい、元の生データを署名検証できない
 */
// https://developer.github.com/webhooks/

#include "webpage.h"
#include "../util.h"
#include "../net.h"

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

	// JSON parse
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

}	// namespace

GithubPage::GithubPage()
{
	m_enabled = config.GetBool({"HttpServer", "GithubHook", "Enabled"});
	m_secret = config.GetStr({"HttpServer", "GithubHook", "Secret"});
}

HttpResponse GithubPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query,
	const PostKeyValueSet &post)
{
	if (!m_enabled) {
		return HttpResponse(404);
	}

	if (method == "GET") {
		json11::Json json;
		{
			mtx_guard lock(m_mtx);
			json = m_last_push;
		}
		return PrintJson(m_last_push);
	}
	else if (method == "POST") {
		// github hook event type
		const auto &event = header.find("X-GitHub-Event");
		// message GUID
		const auto &delivery = header.find("X-GitHub-Delivery");
		// SHA-1 signature
		const auto &signature = header.find("X-Hub-Signature");
		// application/json POST payload
		const auto &payload = post.find("payload");
		if (event == header.end() || delivery == header.end() ||
			signature == header.end() || payload == post.end()) {
			// Bad Request
			return HttpResponse(400);
		}
		const std::string &payload_str = payload->second.DataInMemory;

		// 署名検証
		std::string my_signature = "sha1=";
		Network::ShaDigest digest;
		net.HmacSha1(m_secret.data(), static_cast<int>(m_secret.size()),
			payload_str.data(), payload_str.size(),
			digest);
		my_signature += net.HexEncode(digest, sizeof(digest));
		if (!net.ConstTimeEqual(signature->second, my_signature)) {
			return HttpResponse(400);
		}

		// JSON パースと処理
		json11::Json json;
		HttpResponse resp = ProcessPost(payload_str, json);
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
