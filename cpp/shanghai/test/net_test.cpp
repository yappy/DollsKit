#include <gtest/gtest.h>
#include "../src/net.h"
#include <cstdio>
#include <string>

using namespace shanghai;
using namespace std::string_literals;

TEST(NetTest, simple) {
	std::vector<char> data = net.Download("http://httpbin.org/ip"s);
	EXPECT_GT(data.size(), 16U);
}

TEST(NetTest, notfound404) {
	EXPECT_THROW({
		net.Download("http://httpbin.org/aaaaa"s);
	}, NetworkError);
}
