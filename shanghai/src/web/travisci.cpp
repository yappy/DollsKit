/*
 * Travis CI webhook receiver
 *
 * application/x-www-form-urlencoded のみ使用可能だが、
 * github と違って URL decode 済みの payload value に署名されているのでなんとかなる
 */
// https://docs.travis-ci.com/user/notifications/#configuring-webhook-notifications

#include "travisci.h"
#include "../system/system.h"
#include "../util.h"
#include "../config.h"
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
<title>Travis CI hook</title>
</head>
<body>
<code>{0}</code>
</body>
</html>
)";
	std::string json_str = json.is_null() ? "NO DATA" : json.dump();
	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {util::HtmlEscape(json_str)}));
}

std::string FetchPublicKey()
{
	const std::string url = "https://api.travis-ci.com/config"s;
	const int timeout_sec = 5;

	std::string src = net.Download(url, timeout_sec);
	std::string err;
	auto json = json11::Json::parse(src, err);

	// エラーもここに含める
	const auto &val = json["config"]["notifications"]["webhook"]["public_key"];
	if (!val.is_string()) {
		throw std::runtime_error("Fetching public key failed");
	}
	return val.string_value();
}

HttpResponse ProcessPost(const std::string &json_str, json11::Json &result)
{
	const char *tmpl =
R"(<!DOCTYPE html>
<html lang="en">
<head>
<title>Travis CI hook</title>
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
			util::Format(tmpl, {util::HtmlEscape(err)}));
	}

	// OK
	std::string msg = "Travis CI build: ";
	msg += result["status_message"].string_value();
	msg += '\n';
	msg += result["build_url"].string_value();

	auto task_func = [msg = std::move(msg)]
		(TaskServer &server, const std::atomic<bool> &cancel)
		{
			auto &twitter = system::Get().twitter;
			twitter.Tweet(msg);
		};
	auto &task_queue = system::Get().task_queue;
	task_queue.Enqueue(task_func);

	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {"OK"}));
}

}	// namespace

TravisCiPage::TravisCiPage()
{
	m_enabled = config.GetBool({"HttpServer", "TravisCiHook", "Enabled"});
}

HttpResponse TravisCiPage::Do(
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
			rlock lock(m_mtx);
			json = m_last_build;
		}
		return PrintJson(json);
	}
	else if (method == "POST") {
		// SHA-1 signature
		const auto &signature = header.find("Signature");
		// application/x-www-form-urlencoded POST payload
		const auto &payload = post.find("payload");
		if (signature == header.end() || payload == post.end()) {
			// Bad Request
			return HttpResponse(400);
		}
		const std::string &payload_str = payload->second.DataInMemory;

		// 署名検証
		// TODO
		/*
		Network::ShaDigest digest;
		net.HmacSha1(m_secret.data(), static_cast<int>(m_secret.size()),
			payload_str.data(), payload_str.size(),
			digest);
		my_signature += net.HexEncode(digest, sizeof(digest));
		if (!net.ConstTimeEqual(signature->second, my_signature)) {
			return HttpResponse(400);
		}
		*/

		// JSON パースと処理
		json11::Json json;
		HttpResponse resp = ProcessPost(payload_str, json);
		{
			wlock lock(m_mtx);
			m_last_build = json;
		}
		return resp;
	}
	else {
		return HttpResponse(500);
	}
}

}	// namespace web
}	// namespace shanghai
