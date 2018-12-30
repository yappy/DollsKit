#include <gtest/gtest.h>
#include "../src/util.h"

using namespace shanghai;

TEST(UtilTest, ToInt) {
	EXPECT_EQ(2000000000, util::to_int("2000000000"));
	EXPECT_EQ(-2000000000, util::to_int("-2000000000"));
	EXPECT_THROW(util::to_int("str"), std::runtime_error);
	EXPECT_THROW(util::to_int("3000000000"), std::runtime_error);
	EXPECT_THROW(util::to_int("-3000000000"), std::runtime_error);
}
