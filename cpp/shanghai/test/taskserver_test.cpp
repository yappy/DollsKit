#include <gtest/gtest.h>
#include "../src/taskserver.h"

using namespace shanghai;

TEST(TaskServerTest, thread_pool) {
	ThreadPool pool(4);

	int x = 0;
	auto task = [&x](const std::atomic<bool> &cancel) -> void {
		x = 1;
	};
	auto f = pool.PostTask(task);
	// wait and get
	f.get();
	EXPECT_EQ(1, x);
}
