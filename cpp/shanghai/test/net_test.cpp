#include <gtest/gtest.h>
#include "../src/net.h"
#include "../src/exec.h"
#include <string>
#include <chrono>
#include <thread>

using namespace shanghai;
using namespace std::string_literals;
using namespace std::chrono_literals;

TEST(NetTest, Escape) {
	// https://developer.twitter.com
	// /en/docs/basics/authentication/guides/percent-encoding-parameters
	EXPECT_EQ(
		"Ladies%20%2B%20Gentlemen"s,
		net.Escape("Ladies + Gentlemen"s));
	EXPECT_EQ(
		"An%20encoded%20string%21"s,
		net.Escape("An encoded string!"s));
	EXPECT_EQ(
		"Dogs%2C%20Cats%20%26%20Mice"s,
		net.Escape("Dogs, Cats & Mice"s));
	// showman
	EXPECT_EQ(
		u8"%E2%98%83"s,
		net.Escape("\u2603"s));
}

TEST(NetTest, Base64Encode) {
	std::string in;
	for (int i = 0; i < 19997; i++) {
		in += static_cast<char>(i);
	}

	Process p("/usr/bin/base64"s, {"--wrap=0"});
	p.InputAndClose(in);
	EXPECT_EQ(0, p.WaitForExit());
	const std::string &expect = p.GetOut().c_str();

	std::string actual = net.Base64Encode(
		in.c_str(), static_cast<int>(in.size()));

	EXPECT_EQ(actual, expect);
}

namespace {
void HmacSha1Body(const void *key, size_t key_size,
	const unsigned char *data, size_t data_size,
	const std::string &digest_str)
{
	ASSERT_EQ(Network::ShaDigestLen * 2, static_cast<int>(digest_str.size()));
	Network::ShaDigest expect;
	for (int i = 0; i < Network::ShaDigestLen; i++) {
		expect[i] = std::stoi(digest_str.substr(i * 2, 2), nullptr, 16);
	}

	Network::ShaDigest digest;
	net.HmacSha1(key, key_size, data, data_size, digest);

	EXPECT_EQ(0, memcmp(digest, expect, sizeof(digest)));
}
}

// https://www.ipa.go.jp/security/rfc/RFC2104JA.html
TEST(NetTest, HmacSha1_1) {
	unsigned char key[20];
	memset(key, 0x0b, sizeof(key));
	unsigned char data[] = "Hi There";
	const auto expect = "b617318655057264e28bc0b6fb378c8ef146be00"s;

	HmacSha1Body(key, sizeof(key), data, sizeof(data) - 1, expect);
}

TEST(NetTest, HmacSha1_2) {
	unsigned char key[] = "Jefe";
	unsigned char data[] = "what do ya want for nothing?";
	const auto expect = "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"s;

	HmacSha1Body(key, sizeof(key) - 1, data, sizeof(data) - 1, expect);
}

TEST(NetTest, HmacSha1_3) {
	unsigned char key[20];
	memset(key, 0xaa, sizeof(key));
	unsigned char data[50];
	memset(data, 0xdd, sizeof(data));
	const auto expect = "125d7342b9ac11cd91a39af48aa17b4f63f175d3"s;

	HmacSha1Body(key, sizeof(key), data, sizeof(data), expect);
}

TEST(NetTest, HmacSha1_4) {
	unsigned char key[25];
	for (size_t i = 0; i < sizeof(key); i++) {
		key[i] = i + 1;
	}
	unsigned char data[50];
	memset(data, 0xcd, sizeof(data));
	const auto expect = "4c9007f4026250c6bc8414f9bf50c86c2d7235da"s;

	HmacSha1Body(key, sizeof(key), data, sizeof(data), expect);
}

TEST(NetTest, HmacSha1_5) {
	unsigned char key[20];
	memset(key, 0x0c, sizeof(key));
	unsigned char data[] = "Test With Truncation";
	const auto expect = "4c1a03424b55e07fe7f27be1d58bb9324a9a5a04"s;

	HmacSha1Body(key, sizeof(key), data, sizeof(data) - 1, expect);
}

TEST(NetTest, HmacSha1_6) {
	unsigned char key[80];
	memset(key, 0xaa, sizeof(key));
	unsigned char data[] = "Test Using Larger Than Block-Size Key"
		" - Hash Key First";
	const auto expect = "aa4ae5e15272d00e95705637ce8a3b55ed402112"s;

	HmacSha1Body(key, sizeof(key), data, sizeof(data) - 1, expect);
}

TEST(NetTest, HmacSha1_7) {
	unsigned char key[80];
	memset(key, 0xaa, sizeof(key));
	unsigned char data[] = "Test Using Larger Than Block-Size Key and "
		"Larger Than One Block-Size Data";
	const auto expect = "e8e99d0f45237d786d6bbaa7965c7808bbff1a91"s;

	HmacSha1Body(key, sizeof(key), data, sizeof(data) - 1, expect);
}

TEST(NetTest, OAuthHeader) {
	puts("TODO: temp test!");
}

TEST(NetTest, Simple_SLOW) {
	std::vector<char> data = net.Download("https://httpbin.org/ip"s);
	EXPECT_GT(data.size(), 16U);
}

TEST(NetTest, NotFound404_SLOW) {
	EXPECT_THROW(
		net.Download("https://httpbin.org/aaaaa"s),
		NetworkError);
}

TEST(NetTest, Timeout_SLOW) {
	// 10s delay, 1s timeout
	EXPECT_THROW(
		net.Download("https://httpbin.org/delay/10"s, 1),
		NetworkError);
}

TEST(NetTest, Cancel_SLOW) {
	std::atomic<bool> cancel(false);
	std::thread th([&cancel]() {
		// 1s cancel
		std::this_thread::sleep_for(1s);
		cancel.store(true);
	});

	auto start = std::chrono::system_clock::now();
	// 10s delay
	EXPECT_THROW(
		net.Download("https://httpbin.org/delay/10"s, 0, cancel),
		NetworkError);
	auto end = std::chrono::system_clock::now();
	th.join();

	// < 2s ?
	EXPECT_LT(end - start, 2s);
}

TEST(NetTest, BasicAuth_SLOW) {
	const auto user = "a"s;
	const auto pass = "b"s;
	const auto url = "https://httpbin.org/basic-auth/"s + user + "/"s + pass;
	std::vector<char> data = net.DownloadBasicAuth(url, user, pass);
	EXPECT_GT(data.size(), 0U);
}

TEST(NetTest, BasicAuthFail_SLOW) {
	const auto user = "a"s;
	const auto pass = "b"s;
	const auto url = "https://httpbin.org/basic-auth/user/pass"s;
	EXPECT_THROW(
		net.DownloadBasicAuth(url, user, pass),
		NetworkError);
}
