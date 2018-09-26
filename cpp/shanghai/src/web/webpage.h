#ifndef SHANGHAI_WEB_WABPAGE_H
#define SHANGHAI_WEB_WABPAGE_H

#include "../logger.h"
#include "../config.h"
#include "../system/system.h"

namespace shanghai {
namespace web {

using namespace std::string_literals;
using system::WebPage;
using system::KeyValueSet;
using system::HttpResponse;

class EchoPage : public WebPage {
public:
	EchoPage() = default;
	virtual ~EchoPage() = default;

	HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query) override;
};

class PostPage : public WebPage {
public:
	PostPage() = default;
	virtual ~PostPage() = default;

	HttpResponse Do(
		const std::string &method, const std::string &url_match,
		const KeyValueSet &header, const KeyValueSet &query) override;
};

inline void SetupPages()
{
	system::HttpServer &server = system::Get().Http;

	server.AddPage(std::regex("GET|POST"), std::regex(R"(/echo/\w*)"),
		std::make_shared<EchoPage>());
	server.AddPage(std::regex("GET|POST"), std::regex(R"(/post/\w*)"),
		std::make_shared<PostPage>());
}

inline std::string HtmlEscape(const std::string &src)
{
	std::string dst;
	dst.reserve(src.size());
	for (const char &c : src) {
		switch (c) {
		case '&':
			dst += "&amp;";
			break;
		case '"':
			dst += "&quot";
			break;
		case '\'':
			// since HTML5 spec!
			dst += "&apos";
			break;
		case '<':
			dst += "&lt;";
			break;
		case '>':
			dst += "&gt;";
			break;
		default:
			dst += c;
			break;
		}
	}
	return dst;
}

}	// namespace web
}	// namespace shanghai

#endif	// SHANGHAI_WEB_WABPAGE_H
