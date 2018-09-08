#include <gtest/gtest.h>
#include "../src/config.h"

using namespace shanghai;

const char * const TestSrc = R"({ "a": true, "b": 1, "c": "str" })";

TEST(ConfigTest, parse) {
	Config config;
	config.LoadString(TestSrc);
}
