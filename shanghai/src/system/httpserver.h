/*
References: RFC HTTP spec
https://tools.ietf.org/html/rfc7230
https://tools.ietf.org/html/rfc7231
https://tools.ietf.org/html/rfc7232
https://tools.ietf.org/html/rfc7233
https://tools.ietf.org/html/rfc7234
https://tools.ietf.org/html/rfc7235
*/

#ifndef SHANGHAI_SYSTEM_HTTPSERVER_H
#define SHANGHAI_SYSTEM_HTTPSERVER_H

#include <stddef.h>
#include <stdint.h>
#include <mutex>
#include <unordered_map>
#include <regex>

struct MHD_Daemon;
struct MHD_Connection;

namespace shanghai {
namespace system {

struct PostData final {
	// 真のサイズ (未対応)
	uint64_t Size;
	// メモリに入った場合はそのデータ
	std::string DataInMemory;
	// 大きなデータは tmp ファイルに書き出す (未対応)
	std::string FileName;

	PostData() : Size(0) {}
	~PostData() = default;
};

using KeyValueSet = std::unordered_map<std::string, std::string>;
using PostKeyValueSet = std::unordered_map<std::string, PostData>;

struct HttpResponse final {
	uint32_t Status;
	KeyValueSet Header;
	std::string Body;

	explicit HttpResponse(uint32_t status) :
		HttpResponse(status, {}, "") {}
	HttpResponse(uint32_t status, const std::string &body) :
		HttpResponse(status, {}, body) {}
	HttpResponse(uint32_t status, const KeyValueSet &header) :
		HttpResponse(status, header, "") {}
	HttpResponse(uint32_t status,
		const KeyValueSet &header, const std::string &body) :
		Status(status), Header(header), Body(body) {}

	HttpResponse(const HttpResponse &) = default;
	HttpResponse &operator=(const HttpResponse &) = default;
	HttpResponse(HttpResponse &&) = default;
	~HttpResponse() = default;
};

class WebPage {
public:
	WebPage() = default;
	virtual ~WebPage() = default;
	WebPage(const WebPage &) = delete;
	WebPage &operator =(const WebPage &) = delete;

	virtual HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query,
		const PostKeyValueSet &post) = 0;
};

class HttpServer final {
public:
	HttpServer();
	~HttpServer();

	void AddPage(const std::regex &method, const std::regex &url,
		std::shared_ptr<WebPage> page);

private:
	// 1 connection あたりのメモリリミット
	static const uint32_t MemoryLimit = 32 * 1024;
	// FD, メモリ のリミット
	static const uint32_t MaxConn = 64;
	// コネクションだけ確立して何もせずリソースを消費させる攻撃は NG
	static const uint32_t TimeoutSec = 60;
	// 同一 IP アドレスからの接続数制限
	static const uint32_t IpConnLimit = 16;
	// スレッド数
	static const uint32_t ThreadPoolSize = 4;
	// POST の処理に使うバッファサイズ
	static const uint32_t PostBufferSize = 64 * 1024;

	// POST データ量合計制限
	static const uint64_t PostTotalLimit = 64 * 1024;
	// POST InMemory データ量制限
	static const uint32_t PostMemoryLimit = 64 * 1024;

	// method, url, func
	using Route = std::tuple<std::regex, std::regex, std::shared_ptr<WebPage>>;

	struct MHD_Daemon *m_daemon;
	std::mutex m_mtx;
	std::vector<Route> m_routes;
	std::string m_rewrite;

	HttpResponse ProcessRequest(struct MHD_Connection *connection,
		const std::string &url, const std::string &method,
		const std::string &version, const PostKeyValueSet &post) noexcept;
	static int OnRequest(void *cls, struct MHD_Connection *connection,
		const char *url, const char *method,
		const char *version, const char *upload_data,
		size_t *upload_data_size, void **con_cls) noexcept;
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_HTTPSERVER_H
