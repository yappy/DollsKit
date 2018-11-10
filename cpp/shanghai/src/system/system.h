#ifndef SHANGHAI_SYSTEM_SYSTEM_H
#define SHANGHAI_SYSTEM_SYSTEM_H

#include "twitter.h"
#include "httpserver.h"

namespace shanghai {
// 初期化時、タスク起動前に初期化され、全タスク終了後に破棄されるコンポーネント群
// 再起動時、再初期化される
namespace system {

struct System {
	Twitter twitter;
	HttpServer http_server;
};

void Initialize();
void Finalize() noexcept;
System &Get();

class SafeSystem {
public:
	SafeSystem()
	{
		Initialize();
	}
	~SafeSystem()
	{
		Finalize();
	}
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_SYSTEM_H
