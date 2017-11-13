require 'cgi'
require 'json'
require 'mysql2'
load 'common.rb'

@cgi = CGI.new("html5")

def create_table()
	config = JSON.parse(File.read("../config.json"))
	client = Mysql2::Client.new(
		:encoding => 'utf8mb4',
		:username => config["user"],
		:password => config["pass"],
		:database => config["database"])
	result = client.query("SELECT * FROM #{Const::TABLE_NAME}")

	@cgi.table({ "border" => "1" }) {
		result.collect do |row|
			@cgi.tr {
				row.collect do |k, v|
					#@cgi.td { CGI.escapeHTML(k.to_s) } +
					@cgi.td { CGI.escapeHTML(v.to_s).gsub(/\n/, "<BR>") }
				end.join("")
			}
		end.join("")
	}
end

def create_html(title, body)
	@cgi.html {
		@cgi.head {
			@cgi.title { title }
		} +
		@cgi.body { body }
	}
end

def main()
	header = { "charset" => "utf-8" }
	body = ""

	@cgi.out(header) {
		CGI.pretty(create_html("Push log viewer", create_table))
	}
end

main()

