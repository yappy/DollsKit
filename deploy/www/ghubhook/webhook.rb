require 'tempfile'
require 'cgi'
require 'json'
require 'mysql2'
require 'openssl'
load 'common.rb'

@config = JSON.parse(File.read("../config.json"))

# copy stdin to file
unless File.file?($stdin)
	temp = Tempfile.open('stdin') do |temp|
		$stdin.binmode
		IO.copy_stream($stdin, temp)
		temp
	end
	$stdin = temp.open
	$stdin.binmode
end
# read for signiture check and rewind
@payload_body = $stdin.read
$stdin.rewind

# parse $stdin if POST
@cgi = CGI.new("html5")

def github_push(payload)
	client = Mysql2::Client.new(
		:encoding => 'utf8mb4',
		:username => @config["user"],
		:password => @config["pass"],
		:database => @config["database"])
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

def secure_compare(a, b)
	return false unless a.bytesize == b.bytesize

	l = a.unpack("C*")

	r, i = 0, -1
	b.each_byte { |v| r |= v ^ l[i+=1] }
	r == 0
end

def verify_signature(payload_body, token, github_signature)
	signature = 'sha1=' + OpenSSL::HMAC.hexdigest(
		OpenSSL::Digest.new('sha1'), token, payload_body)
	secure_compare(signature, github_signature)
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
	github_signature = ENV['HTTP_X_HUB_SIGNATURE']

	unless verify_signature(@payload_body, @config["token"],
		github_signature)
		return ["SERVER_ERROR", "<P>Signature error</P>"]
	end

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
