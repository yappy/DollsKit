#include <gtest/gtest.h>
#include <curl/curl.h>
#include <cstdio>
#include <string>

extern "C"
size_t wfunc(void *buffer, size_t size, size_t nmemb, void *userp)
{
	std::printf("%s",
		std::string(static_cast<char *>(buffer), size * nmemb).c_str());
	return nmemb;
};

TEST(CurlTest, simple) {
	CURLcode ret = curl_global_init(CURL_GLOBAL_ALL);
	ASSERT_EQ(0, ret);

	CURL *handle = curl_easy_init();
	ASSERT_NE(nullptr, handle);

	ret = curl_easy_setopt(handle, CURLOPT_URL, "https://www.google.co.jp/");
	EXPECT_EQ(0, ret);

	ret = curl_easy_setopt(handle, CURLOPT_WRITEFUNCTION, wfunc);
	EXPECT_EQ(0, ret);

	ret = curl_easy_setopt(handle, CURLOPT_WRITEDATA, nullptr);
	EXPECT_EQ(0, ret);

	ret = curl_easy_perform(handle);
	EXPECT_EQ(0, ret);

	curl_global_cleanup();
}
