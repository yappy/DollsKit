#include "net.h"
#include <openssl/bio.h>
#include <openssl/evp.h>
#include <curl/curl.h>
#include <memory>
#include <algorithm>

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

std::string Network::Escape(const std::string &str)
{
	SafeCurl curl(::curl_easy_init());
	if (curl == nullptr) {
		throw NetworkError("CURL init handle failed");
	}
	char *buf = ::curl_easy_escape(curl.get(), str.c_str(), str.size());
	if (buf == nullptr) {
		throw std::bad_alloc();
	}
	std::string result(buf);
	::curl_free(buf);
	return result;
}

std::string Network::Base64Encode(const void *buf, int size)
{
	BIO *bio_base64 = BIO_new(BIO_f_base64());
	BIO_set_flags(bio_base64, BIO_FLAGS_BASE64_NO_NL);

	BIO *bio_memout = BIO_new(BIO_s_mem());

	BIO_push(bio_base64, bio_memout);

	BIO_write(bio_base64, buf, size);
	BIO_flush(bio_base64);

	char *p;
	long len = BIO_get_mem_data(bio_memout, &p);
	std::string result(p, len);

	BIO_free_all(bio_base64);

	return result;
}

namespace {
// 受信コールバック
// userp: 格納先 vector<char> へのポインタ
size_t WriteFunc(void *buffer, size_t size, size_t nmemb, void *userp)
{
	auto cbuf = static_cast<char *>(buffer);
	auto data = static_cast<std::vector<char> *>(userp);

	data->insert(data->end(), cbuf, cbuf + size * nmemb);

	return nmemb;
}

// 受信中コールバック
// clientp: atomic<bool> キャンセル変数へのポインタ
int ProgressFunc(void *clientp, curl_off_t dltotal, curl_off_t dlnow,
	curl_off_t ultotal, curl_off_t ulnow)
{
	auto cancel = static_cast<std::atomic<bool> *>(clientp);
	if (cancel->load()) {
		// 転送関数は CURLE_ABORTED_BY_CALLBACK を返す
		return 1;
	}
	return 0;
}
}	// namespace

template <class F>
std::vector<char> Network::DownloadInternal(
	const std::string &url, int timeout_sec,
	const std::atomic<bool> &cancel, F prepair)
{
	SafeCurl curl(::curl_easy_init());
	if (curl == nullptr) {
		throw NetworkError("CURL init handle failed");
	}

	CURLcode ret;
	std::vector<char> data;

	// シグナルは危険なので無効にする
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_NOSIGNAL, 1L);
	CheckError(ret);
	// URL
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_URL, url.c_str());
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
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_XFERINFODATA, &cancel);
	CheckError(ret);
	ret = ::curl_easy_setopt(curl.get(), CURLOPT_NOPROGRESS, 0L);
	CheckError(ret);

	// カスタム処理
	prepair(curl);

	// 開始
	ret = ::curl_easy_perform(curl.get());
	CheckError(ret);

	// HTTP status = 200 番台以外はエラーとする (リダイレクトもエラーになるので注意)
	long http_code;
	ret = ::curl_easy_getinfo(curl.get(), CURLINFO_RESPONSE_CODE, &http_code);
	CheckError(ret);
	if (http_code < 200 || http_code >= 300) {
		throw NetworkError("HTTP failed status: "s + std::to_string(http_code));
	}

	// move
	return data;
}

std::vector<char> Network::Download(const std::string &url, int timeout_sec,
	const std::atomic<bool> &cancel)
{
	return DownloadInternal(url, timeout_sec, cancel, [](const SafeCurl &){});
}

std::vector<char> Network::DownloadBasicAuth(const std::string &url,
	const std::string &user, const std::string &pass,
	int timeout_sec, const std::atomic<bool> &cancel)
{
	return DownloadInternal(url, timeout_sec, cancel,
		[&user, &pass](const SafeCurl &curl) {
			CURLcode ret;

			ret = ::curl_easy_setopt(curl.get(),
				CURLOPT_HTTPAUTH, (long)CURLAUTH_BASIC);
			CheckError(ret);
			ret = ::curl_easy_setopt(curl.get(),
				CURLOPT_USERNAME, user.c_str());
			CheckError(ret);
			ret = ::curl_easy_setopt(curl.get(),
				CURLOPT_PASSWORD, pass.c_str());
			CheckError(ret);
		});
}

// https://developer.twitter.com
// /en/docs/basics/authentication/guides/authorizing-a-request
std::string Network::CreateOAuthField(const std::string &url,
	const std::string &consumer_key)
{
	std::vector<std::pair<std::string, std::string>> param;

	// oauth_consumer_key: アプリの識別子
	param.emplace_back("oauth_consumer_key", consumer_key);

	// oauth_nonce: ランダム値
	// OAuth spec ではリプレイ攻撃対策との記述あり
	// 暗号学的安全性は要らない気もするが一応そうしておく
	// Twitter によるとランダムな英数字なら何でもいいらしいが、例に挙げられている
	// 32byte の乱数を BASE64 にして英数字のみを残したものとする
	std::array<uint8_t, 32> nonce;
	for (auto &b : nonce) {
		b = static_cast<uint8_t>(m_secure_rand());
	}
	std::string nonce_b64 = net.Base64Encode(&nonce, sizeof(nonce));
	std::string nonce_str;
	std::copy_if(nonce_b64.begin(), nonce_b64.end(),
		std::back_inserter(nonce_str),
		[](unsigned char c) { return std::isalnum(c); });
	param.emplace_back("oauth_nonce", nonce_str);

	std::string result;
	bool is_first = true;
	for (const auto &entry : param) {
		if (is_first) {
			is_first = false;
		}
		else {
			result += ", ";
		}
		// escape(key) '=' '"' escape(value) '"'
		// key はエスケープ不要なものしかないので省略
		result += entry.first;
		result += '=';
		result += '"';
		result += Escape(entry.second);
		result += '"';
	}
	return result;
}

/*
std::vector<char> Network::DownloadOAuth(const std::string &url,
	const std::string &consumer_key,
	int timeout_sec, const std::atomic<bool> &cancel);
*/


Network net;

}	// namespace shanghai
