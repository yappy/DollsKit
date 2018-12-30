#include <gtest/gtest.h>
#include "../src/util.h"

using namespace shanghai;

TEST(UtilTest, ToInt) {
	EXPECT_EQ(2000000000, util::to_int("2000000000"));
	EXPECT_EQ(-2000000000, util::to_int("-2000000000"));
	EXPECT_THROW(util::to_int("str"), std::runtime_error);
	EXPECT_THROW(util::to_int(""), std::runtime_error);
	EXPECT_THROW(util::to_int("3000000000"), std::runtime_error);
	EXPECT_THROW(util::to_int("-3000000000"), std::runtime_error);
}

TEST(UtilTest, ToUInt64) {
	EXPECT_EQ(0ULL, util::to_uint64("0"));
	EXPECT_EQ(0xffffffffffffffffULL, util::to_uint64("18446744073709551615"));
	EXPECT_THROW(util::to_uint64("str"), std::runtime_error);
	EXPECT_THROW(util::to_uint64(""), std::runtime_error);
	EXPECT_THROW(util::to_uint64("9999999999999999999999"), std::runtime_error);
}
