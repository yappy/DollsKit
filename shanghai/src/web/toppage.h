#ifndef SHANGHAI_WEB_TOPPAGE_H
#define SHANGHAI_WEB_TOPPAGE_H

#include "../system/httpserver.h"

namespace shanghai {
namespace web {

using namespace system::http;

class TopPage : public WebPage {
public:
	TopPage() = default;
	virtual ~TopPage() = default;

	HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query,
		const PostKeyValueSet &post) override;
};


}	// namespace web
}	// namespace shanghai

#endif	// SHANGHAI_WEB_TOPPAGE_H
