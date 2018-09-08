#include <gtest/gtest.h>
#include "../src/taskserver.h"

using namespace shanghai;

TEST(TaskServerTest, ThreadPool) {
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

TEST(TaskServerTest, ThreadPoolHeavy) {
	ThreadPool pool(4);

	const int Num = 1024;
	std::array<std::future<void>, Num> f;
	std::array<int, Num> x;
	for (int i = 0; i < Num; i++) {
		auto task = [i, &x](const std::atomic<bool> &cancel) -> void {
			x[i] = i;
		};
		f[i] = pool.PostTask(task);
	}
	// wait and get
	for (int i = 0; i < Num; i++) {
		f[i].get();
		EXPECT_EQ(i, x[i]);
	}
}

TEST(TaskServerTest, ThreadPoolException) {
	ThreadPool pool(4);

	auto task = [](const std::atomic<bool> &cancel) -> void {
		throw 1;
	};
	auto f = pool.PostTask(task);
	// wait and get
	EXPECT_THROW({
		f.get();
	}, int);
}
