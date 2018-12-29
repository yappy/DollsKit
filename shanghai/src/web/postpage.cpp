#include "postpage.h"
#include "../util.h"

namespace shanghai {
namespace web {

HttpResponse PostPage::Do(
	const std::string &method, const std::string &url_match,
	const KeyValueSet &header, const KeyValueSet &query,
	const PostKeyValueSet &post)
{
	std::string header_str = "<ul>\n";
	for (const auto &entry : header) {
		header_str += util::Format("  <li>{0}: {1}</li>\n",
			{util::HtmlEscape(entry.first), util::HtmlEscape(entry.second)});
	}
	header_str += "</ul>";

	std::string post_str = "<ul>\n";
	for (const auto &entry : post) {
		post_str += util::Format("  <li>{0}: (file: {1}){2}</li>\n",
			{
				util::HtmlEscape(entry.first),
				util::HtmlEscape(entry.second.FileName),
				util::HtmlEscape(entry.second.DataInMemory)
			});
	}
	post_str += "</ul>";

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

<h2>POST Data</h2>
{3}

<h2>POST Form</h2>
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
		util::Format(tmpl, {method, url_match, header_str, post_str}));
}

}	// namespace web
}	// namespace shanghai
