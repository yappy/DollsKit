#
# Dolls Daemon init script
#

PID_FILE = "dollsd.pid".freeze
STOP_TIMEOUT = 3.0
POLL_PERIOD = 0.1

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

	ok = false
	0.0.step(STOP_TIMEOUT, POLL_PERIOD) do |sec|
		unless File.exist?(PID_FILE) then
			ok = true
			break
		end
		sleep POLL_PERIOD
	end
	puts ok ? "OK" : "NG"
end

def reload
	puts "reload"
	pid = IO.read(PID_FILE).to_i
	Process.kill("HUP", pid)
	puts "OK"
end


case ARGV[0]
when "start" then
	start
when "stop" then
	stop
when "reload" then
	reload
else
	puts "Usage: ruby #{__FILE__} {start|stop|reload}"
	exit false
end
