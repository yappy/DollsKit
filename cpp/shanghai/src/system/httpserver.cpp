#include "httpserver.h"
#include "../logger.h"
#include "../config.h"
#include "../util.h"
#include <microhttpd.h>

namespace shanghai {
namespace system {

namespace {

using mtx_guard = std::lock_guard<std::mutex>;

// safe libmicrohttpd post_processor
struct PostProcessorDeleter {
	void operator()(struct MHD_PostProcessor *p)
	{
		::MHD_destroy_post_processor(p);
	}
};
using SafePostProcessor = std::unique_ptr<
	struct MHD_PostProcessor, PostProcessorDeleter>;

// 1つの POST リクエストの間保持される状態
// (1) libmicrohttpd post_processor
//     "application/x-www-form-urlencoded" or "multipart/form-data" のみ対応
// (2) plain (自力パース、というかパースしない)
//     "application/json"

class RequestContext {
public:
	RequestContext(uint64_t max_total_size, uint32_t max_in_memory_size) :
		MaxTotalSize(max_total_size), MaxInMemorySize(max_in_memory_size),
		m_post_data(), m_http_status(0), m_total_size(0)
	{}
	virtual ~RequestContext() = default;

	// サイズ制限
	const uint64_t MaxTotalSize;
	const uint32_t MaxInMemorySize;

	virtual bool Process(const char *upload_data, size_t upload_data_size) = 0;

	const PostKeyValueSet &PostData() { return m_post_data; }
	bool IsError() { return m_http_status != 0; }
	uint32_t GetHttpError() { return m_http_status; }

protected:
	// 処理結果
	PostKeyValueSet m_post_data;
	// 0以外なら処理中に(切断するほどではない) HTTP エラー
	// 全 POST データを読み切るまでエラーレスポンスを返すことはできない
	uint32_t m_http_status;
	// 現在の処理サイズ合計
	uint64_t m_total_size;
};

class MhdRequestContext : public RequestContext {
public:
	MhdRequestContext(uint64_t max_total_size, uint32_t max_in_memory_size) :
		RequestContext(max_total_size, max_in_memory_size)
	{}

	virtual bool Process(const char *upload_data, size_t upload_data_size)
		override
	{
		return ::MHD_post_process(m_post_proc.get(),
			upload_data, upload_data_size) == MHD_YES;
	}

	bool Initialize(struct MHD_Connection *connection,
		uint32_t post_buffer_size)
	{
		m_post_proc.reset(::MHD_create_post_processor(
			connection, post_buffer_size, ProcessPost, this));
		return m_post_proc != nullptr;
	}

private:
	SafePostProcessor m_post_proc;

	// POST プロセッサのイテレーションコールバック
	static int ProcessPost(void *cls, enum MHD_ValueKind kind, const char *key,
		const char *filename, const char *content_type,
		const char *transfer_encoding, const char *data, uint64_t off, size_t size)
	{
		auto req_ctx = static_cast<MhdRequestContext *>(cls);
		PostKeyValueSet &post_data = req_ctx->m_post_data;

		// この POST 中での総サイズチェック
		req_ctx->m_total_size += size;
		if (req_ctx->m_total_size > req_ctx->MaxTotalSize) {
			// 413 Payload Too Large
			req_ctx->m_http_status = 413;
			return MHD_YES;
		}

		// キーが存在しないならデフォルトコンストラクト
		// 値の文字列に追加する
		auto &value = req_ctx->m_post_data[key];
		value.FileName = (filename == nullptr) ? "" : filename;
		value.Size += size;
		value.DataInMemory.append(data, size);
		if (value.DataInMemory.size() > req_ctx->MaxInMemorySize) {
			value.DataInMemory.resize(req_ctx->MaxInMemorySize);
			// 413 Payload Too Large
			// 巨大ファイルは tmp ファイルに書く必要がある
			req_ctx->m_http_status = 413;
			return MHD_YES;
		}

		return MHD_YES;
	}
};

class PlainRequestContext : public RequestContext {
public:
	PlainRequestContext(uint64_t max_total_size, uint32_t max_in_memory_size) :
		RequestContext(max_total_size, max_in_memory_size)
	{}

	virtual bool Process(const char *upload_data, size_t upload_data_size)
		override
	{
		// この POST 中での総サイズチェック
		m_total_size += upload_data_size;
		if (m_total_size > MaxTotalSize) {
			// 413 Payload Too Large
			m_http_status = 413;
			return true;
		}
		// キーが存在しないならデフォルトコンストラクト
		// 値の文字列に追加する
		auto &value = m_post_data["payload"];
		value.Size += upload_data_size;
		value.DataInMemory.append(upload_data, upload_data_size);
		if (value.DataInMemory.size() > MaxInMemorySize) {
			value.DataInMemory.resize(MaxInMemorySize);
			// 413 Payload Too Large
			// 巨大ファイルは tmp ファイルに書く必要がある
			m_http_status = 413;
			return true;
		}
		return true;
	}
};

// 最初から HTTP エラー状態でデータは読んで即捨てる
class ErrorRequestContext : public RequestContext {
public:
	ErrorRequestContext(uint32_t http_status) : RequestContext(0, 0)
	{
		m_http_status = http_status;
	}

	virtual bool Process(const char *upload_data, size_t upload_data_size)
		override
	{
		return true;
	}
};

// HTTP エラーはなく、データが読めたらエラー切断する
class NotPostRequestContext : public RequestContext {
public:
	NotPostRequestContext() : RequestContext(0, 0) {}

	virtual bool Process(const char *upload_data, size_t upload_data_size)
		override
	{
		return false;
	}
};

// デフォルトエラーページのテンプレート
const char * const ErrorPageTmpl =
R"(<!DOCTYPE html>
<html lang="en">
<head>
<title>Error: {0}</title>
</head>
<body>
<h1>Error: {0}</h1>
Sorry.
</body>
</html>
)";

// libmicrohttpd の内部アサートっぽいので諦めて死ぬ
void AtPanic(void *cls, const char *file, unsigned int line, const char *reason)
{
	logger.Log(LogLevel::Fatal, "libmicrohttpd panic");
	logger.Log(LogLevel::Fatal, "%s:%u %s", file, line, reason);
	std::terminate();
}

// GET イテレートコールバックを map<string, string> に変換する
int IterateToMap(void *cls, enum MHD_ValueKind kind,
	const char *key, const char *value) noexcept
{
	auto &map = *static_cast<KeyValueSet *>(cls);
	value = (value == nullptr) ? "" : value;
	map.emplace(key, value);
	return MHD_YES;
}

// リクエストに関連付けたオブジェクトを解放する
// コネクションが切れる等でも終了する可能性があるのでアクセスハンドラではなく
// このコールバックで行う
void RequestCompleted(void *cls, struct MHD_Connection *connection,
	void **con_cls, enum MHD_RequestTerminationCode toe)
{
	logger.Log(LogLevel::Trace, "RequestCompleted");
	std::unique_ptr<RequestContext> req_ctx(
		static_cast<RequestContext *>(*con_cls));
	// unique_ptr に復帰させてここでデストラクト
}

// HTTP response を送信する
int SendResponse(struct MHD_Connection *connection, HttpResponse &&resp)
{
	// エラー (4xx, 5xx) で body がない場合はここで自動生成する
	if (resp.Status / 100 == 4 || resp.Status / 100 == 5) {
		if (resp.Body.size() == 0) {
			const std::string status_str = std::to_string(resp.Status);
			resp.Header["Content-Type"] = "text/html; charset=utf-8";
			resp.Body = util::Format(ErrorPageTmpl, {status_str});
		}
	}

	// HttpResponse を変換処理してクライアントに返す
	// ソースを確認したが malloc してそこに memcpy しているだけなので
	// const を外しても問題ない
	auto resp_del = [](MHD_Response *r) {
		::MHD_destroy_response(r);
	};
	MHD_Response *tmp = ::MHD_create_response_from_buffer(
		resp.Body.size(), const_cast<char *>(resp.Body.c_str()),
		MHD_RESPMEM_MUST_COPY);
	std::unique_ptr<MHD_Response, decltype(resp_del)> mhd_resp(tmp, resp_del);
	if (mhd_resp == nullptr) {
		logger.Log(LogLevel::Error, "MHD_create_response_from_buffer failed");
		return MHD_NO;
	}
	int ret = ::MHD_queue_response(connection, resp.Status, mhd_resp.get());
	if (ret != MHD_YES) {
		logger.Log(LogLevel::Error, "MHD_queue_response failed");
		return MHD_NO;
	}

	return MHD_YES;
}

// Content-Type 前半のイコール判定
// https://tools.ietf.org/html/rfc7231#section-3.1.1.1
// media-type = type "/" subtype *( OWS ";" OWS parameter )
// 例
// Content-Type: text/html; charset=utf-8
// Content-Type: multipart/form-data; boundary=something
// ignore case
//
inline bool CheckContentType(const char *field, const char *type)
{
	size_t typelen = strlen(type);
	if (strncasecmp(field, type, typelen) != 0) {
		return false;
	}
	char next = field[typelen];
	if (next == '\0' || next == ';' || isspace(next)) {
		return true;
	}
	return false;
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
	m_rewrite = config.GetStr({"HttpServer", "Rewrite"});

	// サーバスタート (失敗時はコンストラクト失敗、デストラクトなし)
	::MHD_set_panic_func(AtPanic, nullptr);
	m_daemon = ::MHD_start_daemon(
		MHD_USE_SELECT_INTERNALLY, port, nullptr, nullptr,
		OnRequest, this,
		MHD_OPTION_CONNECTION_MEMORY_LIMIT, MemoryLimit,
		MHD_OPTION_CONNECTION_LIMIT, MaxConn,
		MHD_OPTION_CONNECTION_TIMEOUT, TimeoutSec,
		MHD_OPTION_NOTIFY_COMPLETED, RequestCompleted, this,
		MHD_OPTION_PER_IP_CONNECTION_LIMIT, IpConnLimit,
		MHD_OPTION_THREAD_POOL_SIZE, ThreadPoolSize,
		MHD_OPTION_LISTENING_ADDRESS_REUSE, 1U,
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

// 整形後のリクエストデータから HttpResponse オブジェクトを返す
HttpResponse HttpServer::ProcessRequest(struct MHD_Connection *connection,
	const std::string &url, const std::string &method,
	const std::string &version, const PostKeyValueSet &post) noexcept
{
	logger.Log(LogLevel::Info, "[HTTP] %s %s %s",
		version.c_str(), method.c_str(), url.c_str());

	// version: HTTP/1.0 HTTP/1.1 以外は "505 HTTP Version Not Supported"
	if (version != "HTTP/1.0" && version != "HTTP/1.1") {
		return HttpResponse(505);
	}

	// HEAD は libmicrohttpd が自動で Response body をカットしてくれるので
	// GET と同じ処理をしてあとは任せる
	const std::string &vmethod = (method == "HEAD") ? "GET"s : method;
	// URL の先頭が一致した場合は消す(簡易 rewrite)
	// TODO: 消せないときは 404 にした方がよさそう
	std::string vurl = url;
	if (m_rewrite.size() > 0 && url.find(m_rewrite) == 0) {
		vurl = vurl.substr(m_rewrite.size());
	}
	logger.Log(LogLevel::Trace, "Rewrite(%s) to: %s",
		m_rewrite.c_str(), vurl.c_str());

	// HTTP request header と query を map に変換する
	KeyValueSet request_header;
	KeyValueSet get_args;
	::MHD_get_connection_values(connection, MHD_HEADER_KIND,
		IterateToMap, &request_header);
	::MHD_get_connection_values(connection, MHD_GET_ARGUMENT_KIND,
		IterateToMap, &get_args);

	// Route list から条件にマッチするものを探して実行する
	std::shared_ptr<WebPage> page = nullptr;
	{
		mtx_guard lock(m_mtx);
		for (const auto &elem : m_routes) {
			const std::regex method_re = std::get<0>(elem);
			const std::regex url_re = std::get<1>(elem);
			if (!std::regex_match(vmethod, method_re)) {
				continue;
			}
			if (!std::regex_match(vurl, url_re)) {
				continue;
			}
			page = std::get<2>(elem);
			break;
		}
	}
	if (page != nullptr) {
		// TODO: URL 全体ではなく部分列を渡す
		return page->Do(vmethod, vurl, request_header, get_args, post);
	}
	// マッチするものがなかった場合は 404 とする
	return HttpResponse(404);
}

const uint64_t HttpServer::PostTotalLimit;
const uint32_t HttpServer::PostMemoryLimit;

// libmicrohttpd からの raw callback
int HttpServer::OnRequest(void *cls, struct MHD_Connection *connection,
	const char *url, const char *method,
	const char *version, const char *upload_data,
	size_t *upload_data_size, void **con_cls) noexcept
{
	const bool is_post = "POST"s == method;

	// 最初の1回は同一リクエスト内で保存される con_cls を生成する
	if (*con_cls == nullptr) {
		std::unique_ptr<RequestContext> req_ctx;
		// method と Content-Type で処理タイプを決定する
		if (is_post) {
			const char *content_type = MHD_lookup_connection_value (
				connection, MHD_HEADER_KIND, MHD_HTTP_HEADER_CONTENT_TYPE);
			content_type = (content_type == nullptr) ? "" : content_type;
			if (CheckContentType(content_type, "application/json")) {
				logger.Log(LogLevel::Trace, "POST plain");
				// plain text processor
				req_ctx = std::make_unique<PlainRequestContext>(
					PostTotalLimit, PostMemoryLimit);
			}
			else if (CheckContentType(content_type,
				"application/x-www-form-urlencoded") ||
				CheckContentType(content_type, "multipart/form-data")) {
				logger.Log(LogLevel::Trace, "POST form");
				// MHD post processor
				auto mhd_req_ctx = std::make_unique<MhdRequestContext>(
					PostTotalLimit, PostMemoryLimit);
				if (!mhd_req_ctx->Initialize(connection, PostBufferSize)) {
					return MHD_NO;
				}
				req_ctx = std::move(mhd_req_ctx);
			}
			else {
				logger.Log(LogLevel::Trace, "POST unknown: %s", content_type);
				// 415 Unsupported Media Type
				req_ctx = std::make_unique<ErrorRequestContext>(415);
			}
		}
		else {
			// OK but fatal if there is upload_data
			req_ctx = std::make_unique<NotPostRequestContext>();
		}
		// unique_ptr の管理から外して libmicrohttpd の管理下に
		*con_cls = (void *)req_ctx.release();
		// レスポンスはせずに返る
		return MHD_YES;
	}

	// 以下、2回目以降

	auto req_ctx = static_cast<RequestContext *>(*con_cls);
	// POST データが残っている
	if (*upload_data_size != 0) {
		if (!req_ctx->Process(upload_data, *upload_data_size)) {
			return MHD_NO;
		}
		*upload_data_size = 0;
		// レスポンスはせずに返る
		return MHD_YES;
	}

	// 以下、upload_data_size == 0 であり POST データの残りなし

	if (req_ctx->IsError()) {
		// POST プロセスハンドラが中断した
		return SendResponse(connection, HttpResponse(req_ctx->GetHttpError()));
	}

	auto self = static_cast<HttpServer *>(cls);
	// non-static に移行
	// HttpResponse オブジェクトを返してもらう
	HttpResponse resp = self->ProcessRequest(
		connection, url, method, version, req_ctx->PostData());

	return SendResponse(connection, std::move(resp));
}

}	// namespace system
}	// namespace shanghai
