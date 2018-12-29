#ifndef SHANGHAI_WEB_ECHOPAGE_H
#define SHANGHAI_WEB_ECHOPAGE_H

#include "../system/httpserver.h"

namespace shanghai {
namespace web {

using namespace system::http;

class EchoPage : public WebPage {
public:
	EchoPage() = default;
	virtual ~EchoPage() = default;

	HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query,
		const PostKeyValueSet &post) override;
};


}	// namespace web
}	// namespace shanghai

#endif	// SHANGHAI_WEB_ECHOPAGE_H
