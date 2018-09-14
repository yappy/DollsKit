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

TEST(NetTest, OAuthHeader) {
	puts("TODO: temp test!");
	puts(net.CreateOAuthField("https://hoge.com"s, "consumekey"s).c_str());
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
