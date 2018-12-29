#include "webpage.h"
#include "../system/system.h"

#include "echopage.h"
#include "postpage.h"
#include "github.h"
#include "travisci.h"

namespace shanghai {
namespace web {

void SetupPages()
{
	auto &server = system::Get().http_server;

	server.AddPage(std::regex("GET|POST"), std::regex(R"(/echo/\w*)"),
		std::make_shared<EchoPage>());
	server.AddPage(std::regex("GET|POST"), std::regex(R"(/post/\w*)"),
		std::make_shared<PostPage>());
	server.AddPage(std::regex("GET|POST"), std::regex(R"(/github/\w*)"),
		std::make_shared<GithubPage>());
	server.AddPage(std::regex("GET|POST"), std::regex(R"(/travisci/\w*)"),
		std::make_shared<TravisCiPage>());
}

}	// namespace web
}	// namespace shanghai
