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

	data->reserve(data->size() + size * nmemb);
	data->insert(data->end(), cbuf, cbuf + size * nmemb);

	return nmemb;
}
}	// namespace

std::vector<char> Network::Download(const std::string &url)
{
	SafeCurl curl(::curl_easy_init());
	if (curl == nullptr) {
		throw NetworkError("CURL init handle failed");
	}

	CURLcode ret;
	std::vector<char> data;

	ret = ::curl_easy_setopt(curl.get(), CURLOPT_URL, url.c_str());
	CheckError(ret);
	ret = curl_easy_setopt(curl.get(), CURLOPT_NOSIGNAL, 1);
	CheckError(ret);
	ret = curl_easy_setopt(curl.get(), CURLOPT_WRITEFUNCTION, WriteFunc);
	CheckError(ret);
	ret = curl_easy_setopt(curl.get(), CURLOPT_WRITEDATA, &data);
	CheckError(ret);

	ret = ::curl_easy_perform(curl.get());
	CheckError(ret);

	long http_code;
	ret = curl_easy_getinfo(curl.get(), CURLINFO_RESPONSE_CODE, &http_code);
	CheckError(ret);
	if (http_code != 200) {
		throw NetworkError("HTTP failed status: "s + std::to_string(http_code));
	}

	// move
	return data;
}


Network net;

}	// namespace shanghai
