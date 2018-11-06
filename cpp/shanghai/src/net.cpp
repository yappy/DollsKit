#include "net.h"
#include <openssl/bio.h>
#include <openssl/evp.h>
#include <openssl/hmac.h>
#include <openssl/sha.h>
#include <curl/curl.h>
#include <memory>
#include <algorithm>
#include <ctime>

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

class SafeSlist {
public:
	SafeSlist() : m_slist(nullptr) {}
	~SafeSlist()
	{
		::curl_slist_free_all(m_slist);
	}
	struct curl_slist *get() const noexcept
	{
		return m_slist;
	}
	void Append(const char *str)
	{
		struct curl_slist *result = ::curl_slist_append(m_slist, str);
		if (result == nullptr) {
			throw NetworkError("slist append failed");
		}
		m_slist = result;
	}

private:
	struct curl_slist *m_slist;
};

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

std::string Network::HexEncode(const void *buf, int size)
{
	static const char table[] = "0123456789abcdef";
	const auto *p = static_cast<const uint8_t *>(buf);
	std::string result;
	for (int i = 0; i < size; i++) {
		uint8_t x = p[i];
		uint8_t hi = (x & 0xf0) >> 4;
		uint8_t lo = (x & 0x0f);
		result += table[hi];
		result += table[lo];
	}
	return result;
}

void Network::HmacSha1(const void *key, int key_len,
	const void *buf, size_t size,
	ShaDigest &result)
{
	static_assert(ShaDigestLen == SHA_DIGEST_LENGTH, "SHA_DIGEST_LENGTH");

	unsigned int reslen = 0;

	unsigned char *ret = HMAC(EVP_sha1(),
		key, key_len, static_cast<const unsigned char *>(buf), size,
		result, &reslen);
	if (ret == nullptr || reslen != ShaDigestLen) {
		throw NetworkError("HMAC-SHA1 error");
	}
}

bool Network::ConstTimeEqual(const void *a, const void *b, size_t len)
{
	return CRYPTO_memcmp(a, b, len) == 0;
}

namespace {
// 受信コールバック
// userp: 格納先 string へのポインタ
size_t WriteFunc(void *buffer, size_t size, size_t nmemb, void *userp)
{
	auto cbuf = static_cast<char *>(buffer);
	auto data = static_cast<std::string *>(userp);

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
std::string Network::DownloadInternal(
	const std::string &url, int timeout_sec,
	const std::atomic<bool> &cancel, F prepair)
{
	SafeCurl curl(::curl_easy_init());
	if (curl == nullptr) {
		throw NetworkError("CURL init handle failed");
	}

	CURLcode ret;
	std::string data;

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

std::string Network::Download(const std::string &url, int timeout_sec,
	const std::atomic<bool> &cancel)
{
	return DownloadInternal(url, timeout_sec, cancel, [](const SafeCurl &){});
}

std::string Network::DownloadBasicAuth(const std::string &url,
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
// /en/docs/basics/authentication/guides/creating-a-signature.html
std::string Network::CalcSignature(
	const std::string &http_method, const std::string &base_url,
	const KeyValue &oauth_param, const KeyValue &query_param,
	const std::string &consumer_secret, const std::string &token_secret)
{
	// "Collecting parameters"
	// percent encode しつつ合成してキーでソートする
	KeyValue param;
	auto encode_insert = [this, &param](const KeyValue &map) {
		for (const auto &entry : map) {
			param.emplace(Escape(entry.first), Escape(entry.second));
		}
	};
	encode_insert(oauth_param);
	encode_insert(query_param);
	// 文字列にする
	// key1=value1&key2=value2&...
	std::string param_str;
	bool is_first = true;
	for (const auto &entry : param) {
		if (is_first) {
			is_first = false;
		}
		else {
			param_str += '&';
		}
		param_str += entry.first;
		param_str += '=';
		param_str += entry.second;
	}

	// "Creating the signature base string"
	// 署名対象
	std::string base = http_method;
	base += '&';
	base += Escape(base_url);
	base += '&';
	base += Escape(param_str);

	// "Getting a signing key"
	// 署名鍵は consumer_secret と token_secret をエスケープして & でつなぐだけ
	std::string key = Escape(consumer_secret);
	key += '&';
	key += Escape(token_secret);

	// "Calculating the signature"
	ShaDigest signature;
	HmacSha1(
		key.data(), key.size(),
		reinterpret_cast<const unsigned char *>(base.data()), base.size(),
		signature);

	return Base64Encode(signature, sizeof(signature));
}

// https://developer.twitter.com
// /en/docs/basics/authentication/guides/authorizing-a-request
Network::KeyValue Network::CreateOAuthField(
	const std::string &consumer_key, const std::string &access_token)
{
	KeyValue param;

	// oauth_consumer_key: アプリの識別子
	param.emplace("oauth_consumer_key", consumer_key);

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
	param.emplace("oauth_nonce", nonce_str);

	// 署名は署名以外のフィールドに対しても行うので後で追加する
	// param.emplace("oauth_signature", sha1(...));

	param.emplace("oauth_signature_method", "HMAC-SHA1");
	param.emplace("oauth_timestamp", std::to_string(std::time(nullptr)));
	param.emplace("oauth_token", access_token);
	param.emplace("oauth_version", "1.0");

	return param;
}

std::string Network::DownloadOAuth(const std::string &base_url,
	const std::string &http_method, const KeyValue &query,
	const std::string &consumer_key, const std::string &access_token,
	const std::string &consumer_secret, const std::string &token_secret,
	int timeout_sec, const std::atomic<bool> &cancel)
{
	// 署名以外の oauth 用のパラメータセットを作る
	KeyValue auth_param = CreateOAuthField(consumer_key, access_token);
	// それと method, URL, query を合わせたものに署名する
	std::string signature = CalcSignature(
		http_method, base_url, auth_param, query,
		consumer_secret, token_secret);
	// 署名を oauth パラメータセットに追加
	auth_param.emplace("oauth_signature", signature);

	// URL にクエリをくっつける
	std::string url = base_url;
	{
		bool is_first = true;
		for (const auto &entry : query) {
			if (is_first) {
				url += '?';
				is_first = false;
			}
			else {
				url += '&';
			}
			url += Escape(entry.first);
			url += '=';
			url += Escape(entry.second);
		}
	}

	// https://developer.twitter.com
	// /en/docs/basics/authentication/guides/authorizing-a-request
	// "Building the header string"
	// Authorization HTTP ヘッダ
	std::string auth_str = "Authorization: OAuth "s;
	{
		bool is_first = true;
		for (const auto &entry : auth_param) {
			if (is_first) {
				is_first = false;
			}
			else {
				auth_str += ", ";
			}
			auth_str += Escape(entry.first);
			auth_str += '=';
			auth_str += '"';
			auth_str += Escape(entry.second);
			auth_str += '"';
		}
	}

	SafeSlist slist;
	slist.Append(auth_str.c_str());
	return DownloadInternal(url, timeout_sec, cancel,
		[&slist](const SafeCurl &curl){
			CURLcode ret;
			ret = curl_easy_setopt(curl.get(), CURLOPT_HTTPHEADER, slist.get());
			CheckError(ret);
		});
}


Network net;

}	// namespace shanghai
