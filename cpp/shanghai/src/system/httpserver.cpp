#include "httpserver.h"
#include "../logger.h"
#include "../config.h"
#include <microhttpd.h>

namespace shanghai {
namespace system {

namespace {

void AtPanic(void *cls, const char *file, unsigned int line, const char *reason)
{
	logger.Log(LogLevel::Fatal, "libmicrohttpd panic");
	logger.Log(LogLevel::Fatal, "%s:%u %s", file, line, reason);
	std::terminate();
}

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

	int port = config.GetInt({"HttpServer", "Port"});
	if (port < 0 || port > 0xffff) {
		throw ConfigError("Invalid HttpServer port");
	}

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
	::MHD_stop_daemon(m_daemon);
	m_daemon = nullptr;
}

HttpResponse HttpServer::ProcessRequest(struct MHD_Connection *connection,
	const std::string &url, const std::string &method,
	const std::string &version, const char *upload_data,
	size_t *upload_data_size, void **con_cls) noexcept
{
	logger.Log(LogLevel::Info, "[HTTP] %s %s %s",
		version.c_str(), method.c_str(), url.c_str());

	// version: "505 HTTP Version Not Supported"
	if (version != "HTTP/1.1") {
		return HttpResponse(505);
	}

	// method: GET, HEAD, POST 以外は "501 Not Implemented"
	if (method != "GET" && method != "HEAD" && method != "POST") {
		return HttpResponse(501);
	}

	// HTTP request header と query を map に変換する
	KeyValueSet request_header;
	KeyValueSet get_args;
	::MHD_get_connection_values(connection, MHD_HEADER_KIND,
		IterateToMap, &request_header);
	::MHD_get_connection_values(connection, MHD_GET_ARGUMENT_KIND,
		IterateToMap, &get_args);
	for (const auto &entry : request_header) {
		logger.Log(LogLevel::Trace, "%s: %s",
			entry.first.c_str(), entry.second.c_str());
	}
	for (const auto &entry : get_args) {
		logger.Log(LogLevel::Trace, "%s=%s",
			entry.first.c_str(), entry.second.c_str());
	}

	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		"<!DOCTYPE html>\n<html><body>Hello</body></html>\n");
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
