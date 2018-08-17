#include <gtest/gtest.h>
#include <json11.hpp>

TEST(JsonTest, simple) {
	const char *sample = R"({"user_id": 123, "name": "Alice"})";
	std::string err;
	auto json = json11::Json::parse(sample, err);

	EXPECT_EQ("", err);
	EXPECT_EQ(123, json["user_id"].int_value());
	EXPECT_EQ("Alice", json["name"].string_value());
}
