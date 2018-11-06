#ifndef SHANGHAI_NET_H
#define SHANGHAI_NET_H

#include <cinttypes>
#include <stdexcept>
#include <atomic>
#include <map>
#include <random>

// MINGW32 実装では std::random_device が暗号論的に安全でない
// https://cpprefjp.github.io/reference/random/random_device.html
#ifdef __MINGW32__
#error MINGW std::random_device is not cryptographically secure.
#endif

namespace shanghai {

class NetworkError : public std::runtime_error {
public:
	NetworkError(const char *msg) : runtime_error(msg) {}
	NetworkError(const std::string &msg) : runtime_error(msg) {}
};

class Network final {
public:
	static const int ShaDigestLen = 20;
	using ShaDigest = unsigned char[ShaDigestLen];

	using KeyValue = std::map<std::string, std::string>;

	Network();
	~Network();
	Network(const Network &) = delete;
	Network & operator=(const Network &) = delete;

	// curl_easy_espace
	// スペースは "+" ではなく "%20" になるのでやや安心
	std::string Escape(const std::string &str);
	// BASE64 改行無し
	std::string Base64Encode(const void *buf, int size);
	// 16進小文字
	std::string HexEncode(const void *buf, int size);
	// HMAC-SHA1
	void HmacSha1(const void *key, int key_len,
		const void *buf, size_t size,
		ShaDigest &result);
	// かかる時間が内容に関わらず len にのみ依存する安全なメモリ比較
	bool ConstTimeEqual(const void *a, const void *b, size_t len);
	inline bool ConstTimeEqual(const std::string &a, const std::string &b)
	{
		if (a.size() != b.size()) {
			return false;
		}
		return ConstTimeEqual(a.data(), b.data(), a.size());
	}

	// 完了するまでブロックする
	// タイムアウトは 0 で無限待ち
	std::string Download(const std::string &url, int timeout_sec = 0,
		const std::atomic<bool> &cancel = std::atomic<bool>(false));
	// BASIC
	std::string DownloadBasicAuth(const std::string &url,
		const std::string &user, const std::string &pass,
		int timeout_sec = 0,
		const std::atomic<bool> &cancel = std::atomic<bool>(false));
	// OAuth 1.0a
	std::string CalcSignature(
		const std::string &http_method, const std::string &base_url,
		const KeyValue &oauth_param, const KeyValue &query_param,
		const std::string &consumer_secret, const std::string &token_secret);
	KeyValue CreateOAuthField(
		const std::string &consumer_key, const std::string &access_token);
	// URL の終わりにつく query (?a=b&c=d...) は署名が必要なため
	// url に含めず query に渡すこと
	std::string DownloadOAuth(const std::string &base_url,
		const std::string &http_method, const KeyValue &query,
		const std::string &consumer_key, const std::string &access_token,
		const std::string &consumer_secret, const std::string &token_secret,
		int timeout_sec = 0,
		const std::atomic<bool> &cancel = std::atomic<bool>(false));

private:
	std::random_device m_secure_rand;

	template <class F>
	std::string DownloadInternal(const std::string &url, int timeout_sec,
		const std::atomic<bool> &cancel, F prepair);
};

extern Network net;

}	// namespace shanghai

#endif	// SHANGHAI_NET_H
