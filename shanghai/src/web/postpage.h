#ifndef SHANGHAI_WEB_POSTPAGE_H
#define SHANGHAI_WEB_POSTPAGE_H

#include "../system/httpserver.h"

namespace shanghai {
namespace web {

using namespace system::http;

class PostPage : public WebPage {
public:
	PostPage() = default;
	virtual ~PostPage() = default;

	HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query,
		const PostKeyValueSet &post) override;
};

}	// namespace web
}	// namespace shanghai

#endif	// SHANGHAI_WEB_POSTPAGE_H
