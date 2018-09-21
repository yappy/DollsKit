#ifndef SHANGHAI_SYSTEM_HTTPSERVER_H
#define SHANGHAI_SYSTEM_HTTPSERVER_H

#include <stddef.h>
#include <stdint.h>

struct MHD_Daemon;
struct MHD_Connection;

namespace shanghai {
namespace system {

class HttpServer {
public:
	HttpServer();
	~HttpServer();

private:
	// 1 connection あたりのメモリリミット
	static const uint32_t MemoryLimit = 32 * 1024;
	// FD, メモリ のリミット
	static const uint32_t MaxConn = 64;
	// コネクションだけ確立して何もせずリソースを消費させる攻撃は NG
	static const uint32_t TimeoutSec = 10;
	// 同一 IP アドレスからの接続数制限
	static const uint32_t IpConnLimit = 16;
	// スレッド数
	static const uint32_t ThreadPoolSize = 4;

	struct MHD_Daemon *m_daemon;

	static int Answer(void *cls, struct MHD_Connection *connection,
		const char *url, const char *method,
		const char *version, const char *upload_data,
		size_t *upload_data_size, void **con_cls) noexcept;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_HTTPSERVER_H
