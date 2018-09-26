#include "webpage.h"
#include "../util.h"

namespace shanghai {
namespace web {

HttpResponse PostPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query)
{
	std::string header_str = "<ul>\n";
	for (const auto &entry : header) {
		header_str += util::Format("  <li>{0}: {1}</li>\n",
			{HtmlEscape(entry.first), HtmlEscape(entry.second)});
	}
	header_str += "</ul>";

	const char *tmpl =
R"(<!DOCTYPE html>

<html lang="en">
<head>
<title>Post Test</title>
</head>

<body>
<h1>Post Test</h1>
<h2>HTTP Request</h2>
<ul>
  <li>Method = {0}</li>
  <li>URL = {1}</li>
</ul>

<h2>HTTP Header</h2>
{2}

<form action="" method="post" enctype="multipart/form-data">
  <input type="text" name="name" />
  <input type="file" name="datafile" />
  <input type="submit" />
</form>

</body>
</html>
)";
	return HttpResponse(200,
		{{"Content-Type", "text/html; charset=utf-8"}},
		util::Format(tmpl, {method, url_match, header_str}));
}

}	// namespace web
}	// namespace shanghai
