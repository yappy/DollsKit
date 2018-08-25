#include "net.h"
#include <curl/curl.h>
#include <memory>

namespace shanghai {

namespace {

using namespace std::string_literals;

struct CurlDeleter {
	void operator()(CURL *curl)
	{
		curl_easy_cleanup(curl);
	}
};
using SafeCurl = std::unique_ptr<CURL, CurlDeleter>;

void CheckError(CURLcode code)
{
	if (code != CURLE_OK) {
		throw NetworkError(::curl_easy_strerror(code));
	}
}

}	// namespace


Network::Network()
{
	CURLcode ret = ::curl_global_init(CURL_GLOBAL_ALL);
	if (ret != 0) {
		throw NetworkError("CURL init failed");
	}
}

Network::~Network()
{
	::curl_global_cleanup();
}

namespace {
extern "C"
size_t WriteFunc(void *buffer, size_t size, size_t nmemb, void *userp)
{
	auto cbuf = static_cast<char *>(buffer);
	auto data = static_cast<std::vector<char> *>(userp);

	data->insert(data->end(), cbuf, cbuf + size * nmemb);

	return nmemb;
}

extern "C"
int ProgressFunc(void *clientp,   curl_off_t dltotal,   curl_off_t dlnow,   curl_off_t ultotal,   curl_off_t ulnow)
{
	auto cancel = static_cast<std::atomic<bool> *>(clientp);
	if (cancel->load()) {
		// 転送関数は CURLE_ABORTED_BY_CALLBACK を返す
		return 1;
	}
	return 0;
}
}	// namespace

std::vector<char> Network::Download(const std::string &url, int timeout_sec,
	const std::atomic<bool> &cancel)
{
	SafeCurl curl(::curl_easy_init());
	if (curl == nullptr) {
		throw NetworkError("CURL init handle failed");
	}

	CURLcode ret;
	std::vector<char> data;

	// URL
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_URL, url.c_str());
	CheckError(ret);
	// シグナルは危険なので無効にする
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_NOSIGNAL, 1L);
	CheckError(ret);
	// タイムアウト(全体)
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_TIMEOUT,
		static_cast<long>(timeout_sec));
	// データ受信コールバックと引数
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_WRITEFUNCTION, WriteFunc);
	CheckError(ret);
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_WRITEDATA, &data);
	CheckError(ret);
	// 受信進捗コールバックと引数、有効化
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_XFERINFOFUNCTION, ProgressFunc);
	CheckError(ret);
	ret = curl_easy_setopt(curl.get(), CURLOPT_XFERINFODATA, &cancel);
	ret = curl_easy_setopt(curl.get(), CURLOPT_NOPROGRESS, 0L);

	// 開始
	ret = ::curl_easy_perform(curl.get());
	CheckError(ret);

	// HTTP status = 200 番台以外はエラーとする (リダイレクトもエラーになるので注意)
	long http_code;
	ret = curl_easy_getinfo(curl.get(), CURLINFO_RESPONSE_CODE, &http_code);
	CheckError(ret);
	if (http_code < 200 || http_code >= 300) {
		throw NetworkError("HTTP failed status: "s + std::to_string(http_code));
	}

	// move
	return data;
}


Network net;

}	// namespace shanghai
