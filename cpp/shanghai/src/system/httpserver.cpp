#include "httpserver.h"
#include "../logger.h"
#include "../config.h"
#include <microhttpd.h>

namespace shanghai {
namespace system {

namespace {

using mtx_guard = std::lock_guard<std::mutex>;

// libmicrohttpd の内部アサートっぽいので諦めて死ぬ
void AtPanic(void *cls, const char *file, unsigned int line, const char *reason)
{
	logger.Log(LogLevel::Fatal, "libmicrohttpd panic");
	logger.Log(LogLevel::Fatal, "%s:%u %s", file, line, reason);
	std::terminate();
}

// イテレートコールバックを map<string, string> に変換する
int IterateToMap(void *cls, enum MHD_ValueKind kind,
	const char *key, const char *value) noexcept
{
	auto &map = *static_cast<KeyValueSet *>(cls);
	value = (value == nullptr) ? "" : value;
	map.emplace(key, value);
	return MHD_YES;
}

}	// namespace

HttpServer::HttpServer() : m_daemon(nullptr)
{
	logger.Log(LogLevel::Info, "Initialize HttpServer...");

	// 設定の読み出し
	int port = config.GetInt({"HttpServer", "Port"});
	if (port < 0 || port > 0xffff) {
		throw ConfigError("Invalid HttpServer port");
	}

	// サーバスタート (失敗時はコンストラクト失敗、デストラクトなし)
	::MHD_set_panic_func(AtPanic, nullptr);
	m_daemon = ::MHD_start_daemon(
		MHD_USE_SELECT_INTERNALLY, port, nullptr, nullptr,
		OnRequest, this,
		MHD_OPTION_CONNECTION_MEMORY_LIMIT, MemoryLimit,
		MHD_OPTION_CONNECTION_LIMIT, MaxConn,
		MHD_OPTION_CONNECTION_TIMEOUT, TimeoutSec,
		MHD_OPTION_PER_IP_CONNECTION_LIMIT, IpConnLimit,
		MHD_OPTION_THREAD_POOL_SIZE, ThreadPoolSize,
		MHD_OPTION_LISTENING_ADDRESS_REUSE,
		MHD_OPTION_END);
	if (m_daemon == nullptr) {
		throw std::runtime_error("Starting HTTP server failed");
	}

	logger.Log(LogLevel::Info, "Initialize HttpServer OK (port=%d)", port);
}

HttpServer::~HttpServer()
{
	// デストラクタでサーバ停止
	::MHD_stop_daemon(m_daemon);
	m_daemon = nullptr;
}

void HttpServer::AddPage(const std::regex &method, const std::regex &url,
	std::shared_ptr<WebPage> page)
{
	mtx_guard lock(m_mtx);
	m_routes.emplace_back(method, url, page);
	// unlock
}

HttpResponse HttpServer::ProcessRequest(struct MHD_Connection *connection,
	const std::string &url, const std::string &method,
	const std::string &version, const char *upload_data,
	size_t *upload_data_size, void **con_cls) noexcept
{
	logger.Log(LogLevel::Info, "[HTTP] %s %s %s",
		version.c_str(), method.c_str(), url.c_str());

	// version: HTTP/1.1 以外は "505 HTTP Version Not Supported"
	if (version != "HTTP/1.1") {
		return HttpResponse(505);
	}

	// HEAD は libmicrohttpd が自動で Response body をカットしてくれるので
	// GET と同じ処理をしてあとは任せる
	const std::string &vmethod = (method == "HEAD") ? "GET"s : method;

	// HTTP request header と query を map に変換する
	KeyValueSet request_header;
	KeyValueSet get_args;
	::MHD_get_connection_values(connection, MHD_HEADER_KIND,
		IterateToMap, &request_header);
	::MHD_get_connection_values(connection, MHD_GET_ARGUMENT_KIND,
		IterateToMap, &get_args);

	std::shared_ptr<WebPage> page = nullptr;
	{
		mtx_guard lock(m_mtx);
		for (const auto &elem : m_routes) {
			const std::regex method_re = std::get<0>(elem);
			const std::regex url_re = std::get<1>(elem);
			if (!std::regex_match(vmethod, method_re)) {
				continue;
			}
			if (!std::regex_match(url, url_re)) {
				continue;
			}
			page = std::get<2>(elem);
			break;
		}
	}
	if (page != nullptr) {
		return page->Do(vmethod, url, request_header, get_args);
	}
	// マッチするものがなかった場合は 404 とする
	return HttpResponse(404);
}

int HttpServer::OnRequest(void *cls, struct MHD_Connection *connection,
	const char *url, const char *method,
	const char *version, const char *upload_data,
	size_t *upload_data_size, void **con_cls) noexcept
{
	auto self = static_cast<HttpServer *>(cls);

	// non-static に移行
	// HttpResponse オブジェクトを返してもらう
	HttpResponse resp = self->ProcessRequest(
		connection, url, method, version,
		upload_data, upload_data_size, con_cls);

	// HttpResponse を変換処理してクライアントに返す
	// ソースを確認したが malloc してそこに memcpy しているだけなので
	// const を外しても問題ない
	auto resp_del = [](MHD_Response *r) {
		::MHD_destroy_response(r);
	};
	MHD_Response *tmp = MHD_create_response_from_buffer(
		resp.Body.size(), const_cast<char *>(resp.Body.c_str()),
		MHD_RESPMEM_MUST_COPY);
	std::unique_ptr<MHD_Response, decltype(resp_del)> mhd_resp(tmp, resp_del);
	if (mhd_resp == nullptr) {
		logger.Log(LogLevel::Error, "MHD_create_response_from_buffer failed");
		return MHD_NO;
	}
	int ret = MHD_queue_response(connection, resp.Status, mhd_resp.get());
	if (ret != MHD_YES) {
		logger.Log(LogLevel::Error, "MHD_queue_response failed");
		return MHD_NO;
	}

	return MHD_YES;
}

}	// namespace system
}	// namespace shanghai
