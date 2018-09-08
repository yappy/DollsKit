#include <gtest/gtest.h>
#include "../src/exec.h"

using namespace shanghai;
using namespace std::string_literals;

TEST(ExecTest, Simple) {
	Process p("/bin/uname"s, {});
	p.WaitForExit();
}

TEST(ExecTest, StdInOut) {
	const auto teststr = "hello, shanghai\n"s;

	Process p("/bin/cat"s, {});
	p.InputAndClose(teststr);
	p.WaitForExit();
	EXPECT_EQ(teststr, p.GetOut());
}
