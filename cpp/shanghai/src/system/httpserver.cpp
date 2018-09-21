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
		Answer, this,
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

int HttpServer::Answer(void *cls, struct MHD_Connection *connection,
	const char *url, const char *method,
	const char *version, const char *upload_data,
	size_t *upload_data_size, void **con_cls) noexcept
{
	auto self = static_cast<HttpServer *>(cls);
	logger.Log(LogLevel::Info, "[HTTP] %s %s", method, url);

	char resbuf[] = "Hello\n";
	MHD_Response *response = MHD_create_response_from_buffer(
		sizeof(resbuf) - 1, resbuf, MHD_RESPMEM_MUST_COPY);
	if (response == nullptr) {
		return MHD_NO;
	}
	int ret = MHD_queue_response(connection, MHD_HTTP_OK, response);
	MHD_destroy_response(response);

	return MHD_YES;
}

}	// namespace system
}	// namespace shanghai
