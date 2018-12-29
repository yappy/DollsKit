#ifndef SHANGHAI_WEB_GITHUB_H
#define SHANGHAI_WEB_GITHUB_H

#include "../system/httpserver.h"
#include <json11.hpp>
#include <shared_mutex>

namespace shanghai {
namespace web {

using namespace system::http;
using wlock = std::lock_guard<std::shared_timed_mutex>;
using rlock = std::shared_lock<std::shared_timed_mutex>;

class GithubPage : public WebPage {
public:
	GithubPage();
	virtual ~GithubPage() = default;

	HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query,
		const PostKeyValueSet &post) override;

private:
	std::shared_timed_mutex m_mtx;
	json11::Json m_last_push;
	bool m_enabled;
	std::string m_secret;
};

}	// namespace web
}	// namespace shanghai

#endif	// SHANGHAI_WEB_GITHUB_H
