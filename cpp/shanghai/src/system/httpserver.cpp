#include "httpserver.h"
#include "../logger.h"
#include "../config.h"
#include "../util.h"
#include <microhttpd.h>

namespace shanghai {
namespace system {

namespace {

using mtx_guard = std::lock_guard<std::mutex>;

struct PostProcessorDeleter {
	void operator()(struct MHD_PostProcessor *p)
	{
		::MHD_destroy_post_processor(p);
	}
};
using SafePostProcessor = std::unique_ptr<
	struct MHD_PostProcessor, PostProcessorDeleter>;

struct RequestContext {
	RequestContext() : post_proc(nullptr) {}

	SafePostProcessor post_proc;
};

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

// イテレートコールバックを map<string, string> に変換する
int IterateToMap(void *cls, enum MHD_ValueKind kind,
	const char *key, const char *value) noexcept
{
	auto &map = *static_cast<KeyValueSet *>(cls);
	value = (value == nullptr) ? "" : value;
	map.emplace(key, value);
	return MHD_YES;
}

// POST プロセッサのイテレーションコールバック
int ProcessPost(void *coninfo_cls, enum MHD_ValueKind kind, const char *key,
	const char *filename, const char *content_type,
	const char *transfer_encoding, const char *data, uint64_t off, size_t size)
{
	// TODO
	logger.Log(LogLevel::Trace,
		"key=%s filename=%s content_type=%s transfer_encoding=%s "
		"off=%llu size=%zu",
		key, filename, content_type, transfer_encoding,
		static_cast<unsigned long long>(off), size);
	return MHD_YES;
}

// リクエストに関連付けたオブジェクトを解放する
// コネクションが切れる等でも終了する可能性があるのでアクセスハンドラではなく
// このコールバックで行う
void RequestCompleted(void *cls, struct MHD_Connection *connection,
	void **con_cls, enum MHD_RequestTerminationCode toe)
{
	logger.Log(LogLevel::Trace, "RequestCompleted");
	std::unique_ptr<RequestContext> req_cxt(
		static_cast<RequestContext *>(*con_cls));
	// unique_ptr に復帰させてここでデストラクト
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

HttpResponse HttpServer::ProcessRequest(struct MHD_Connection *connection,
	const std::string &url, const std::string &method,
	const std::string &version, const char *upload_data,
	size_t *upload_data_size, void **con_cls) noexcept
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
		return page->Do(vmethod, vurl, request_header, get_args);
	}
	// マッチするものがなかった場合は 404 とする
	return HttpResponse(404);
}

int HttpServer::OnRequest(void *cls, struct MHD_Connection *connection,
	const char *url, const char *method,
	const char *version, const char *upload_data,
	size_t *upload_data_size, void **con_cls) noexcept
{
	const bool is_post = "POST"s == method;

	// 最初の1回は同一リクエスト内で保存される con_cls を生成する
	if (*con_cls == nullptr) {
		auto req_ctx = std::make_unique<RequestContext>();
		if (is_post) {
			// POST プロセッサを作成する
			req_ctx->post_proc.reset(::MHD_create_post_processor(
				connection, PostBufferSize, ProcessPost, req_ctx.get()));
			if (req_ctx->post_proc == nullptr) {
				return MHD_NO;
			}
		}
		// ここまで来れたら unique_ptr の管理から外して con_cls に代入
		*con_cls = (void *)req_ctx.release();
		// レスポンスはせずに返る
		return MHD_YES;
	}

	// 以下、2回目以降

	if (is_post) {
		auto req_ctx = static_cast<RequestContext *>(*con_cls);
		// POST データの処理中
		if (*upload_data_size != 0) {
			// POST プロセッサを呼び出す (ProcessPost() が複数回呼ばれる)
			int ret = ::MHD_post_process(req_ctx->post_proc.get(),
				upload_data, *upload_data_size);
			if (ret != MHD_YES) {
				return MHD_NO;
			}
			*upload_data_size = 0;
			// レスポンスはせずに返る
			return MHD_YES;
		}
		// upload_data_size == 0 なら処理完了しているので次に進む
	}

	auto self = static_cast<HttpServer *>(cls);
	// non-static に移行
	// HttpResponse オブジェクトを返してもらう
	HttpResponse resp = self->ProcessRequest(
		connection, url, method, version,
		upload_data, upload_data_size, con_cls);
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
