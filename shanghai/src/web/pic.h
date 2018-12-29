#ifndef SHANGHAI_WEB_PIC_H
#define SHANGHAI_WEB_PIC_H

#include "../system/httpserver.h"

namespace shanghai {
namespace web {

using namespace system::http;

class PicPage : public WebPage {
public:
	PicPage() = default;
	virtual ~PicPage() = default;

	HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query,
		const PostKeyValueSet &post) override;
};


}	// namespace web
}	// namespace shanghai

#endif	// SHANGHAI_WEB_PIC_H
