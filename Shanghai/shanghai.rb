#
# Dolls Daemon init script
#

PID_FILE = "dollsd.pid".freeze

def start
	puts "start"
	# create pid file (fail if already exists)
	open(PID_FILE, IO::WRONLY | IO::CREAT | IO::EXCL)
	# fork and detach daemon
	success = nil
	begin
		success = system("ruby", "dollsd.rb",
			"--daemon", "--pid-file=#{PID_FILE}")
	ensure
		# delete if parent exit code is not 0 or exception
		File.delete(PID_FILE) unless success
	end
	puts success ? "OK" : "NG"
end

def stop
	puts "stop"
	pid = IO.read(PID_FILE).to_i
	Process.kill("TERM", pid)
	puts "OK"
end


case ARGV[0]
when "start" then
	start
when "stop" then
	stop
when "restart" then
	stop
	start
else
	puts "Usage: ruby #{__FILE__} {start|stop}"
	exit false
end
