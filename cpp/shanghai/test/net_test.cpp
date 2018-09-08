#include <gtest/gtest.h>
#include "../src/net.h"
#include <string>
#include <chrono>
#include <thread>

using namespace shanghai;
using namespace std::string_literals;
using namespace std::chrono_literals;

TEST(NetTest, Simple) {
	std::vector<char> data = net.Download("https://httpbin.org/ip"s);
	EXPECT_GT(data.size(), 16U);
}

TEST(NetTest, NotFound404) {
	EXPECT_THROW(
		net.Download("https://httpbin.org/aaaaa"s),
		NetworkError);
}

TEST(NetTest, Timeout) {
	// 10s delay, 1s timeout
	EXPECT_THROW(
		net.Download("https://httpbin.org/delay/10"s, 1),
		NetworkError);
}

TEST(NetTest, Cancel) {
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

TEST(NetTest, BasicAuth) {
	const auto user = "a"s;
	const auto pass = "b"s;
	const auto url = "https://httpbin.org/basic-auth/"s + user + "/"s + pass;
	std::vector<char> data = net.DownloadBasicAuth(url, user, pass);
	EXPECT_GT(data.size(), 0U);
}

TEST(NetTest, BasicAuthFail) {
	const auto user = "a"s;
	const auto pass = "b"s;
	const auto url = "https://httpbin.org/basic-auth/user/pass"s;
	EXPECT_THROW(
		net.DownloadBasicAuth(url, user, pass),
		NetworkError);
}
