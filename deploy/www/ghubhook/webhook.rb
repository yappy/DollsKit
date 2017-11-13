require 'cgi'
require 'json'
require 'mysql2'
load 'common.rb'

@cgi = CGI.new("html5")

def github_push(payload)
	config = JSON.parse(File.read("../config.json"))
	client = Mysql2::Client.new(
		:encoding => 'utf8mb4',
		:username => config["user"],
		:password => config["pass"],
		:database => config["database"])
	statement = client.prepare("INSERT INTO #{Const::TABLE_NAME}" \
		"(ref, hash_before, hash_after, compare, repo_fname, head_msg) " \
		"VALUES (?, ?, ?, ?, ?, ?)")
	statement.execute(
		payload["ref"],
		payload["before"],
		payload["after"],
		payload["compare"],
		payload["repository"]["full_name"],
		payload["head_commit"]["message"]);
end

def create_html(title, body)
	@cgi.html {
		@cgi.head {
			@cgi.title { title }
		} +
		@cgi.body { body }
	}
end

def get()
	["OK", create_html("hook page",
		"<P>GET acccess OK. Please access with POST.</P>")]
end

def post()
	github_delivery = ENV["HTTP_X_GITHUB_DELIVERY"]
	github_event = ENV["HTTP_X_GITHUB_EVENT"]

	case github_event
	when "ping" then
		["OK", "ping"]
	when "push" then
		json = JSON.parse(@cgi["payload"])
		github_push(json)
		["OK", "<P>OK</P>"]
	else
		["BAD_REQUEST", ""]
	end
end

def main()
	header = { "charset" => "utf-8" }
	body = ""

	case ENV["REQUEST_METHOD"]
	when "GET" then
		header["status"], body = get()
	when "POST" then
		header["status"], body = post()
	else
		header["status"], body = "BAD_REQUEST", ""
	end

	@cgi.out(header){ CGI.pretty(body) }
end

main()
