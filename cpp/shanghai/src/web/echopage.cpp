#include "webpage.h"
#include "../util.h"

namespace shanghai {
namespace web {

HttpResponse EchoPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query)
{
	std::string header_str = "<ul>\n";
	for (const auto &entry : header) {
		header_str += util::Format("  <li>{0}: {1}</li>\n",
			{entry.first, entry.second});
	}
	header_str += "</ul>";

	std::string query_str = "<ul>\n";
	for (const auto &entry : query) {
		query_str += util::Format("  <li>{0}: {1}</li>\n",
			{entry.first, entry.second});
	}
	query_str += "</ul>";

	const char *tmpl =
R"(<!DOCTYPE html>

<html lang="en">
<head>
<title>Echo Test</title>
</head>

<body>
<h1>Echo Test</h1>
<h2>HTTP Request</h2>
<ul>
  <li>Method = {0}</li>
  <li>URL = {1}</li>
</ul>

<h2>HTTP Header</h2>
{2}

<h2>GET Query String</h2>
{3}

</body>
</html>
)";
	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {method, url_match, header_str, query_str}));
}

}	// namespace web
}	// namespace shanghai
