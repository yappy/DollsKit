# Reboot script for auto update

require 'date'

# constants
# waiting old process timeout
WAIT_SEC = 60

# Don't 'real' redirect
# Updated process should output to the same console as
# parent (= before update) process
$stdout = File.open("reboot_out.txt", "w")
$stderr = File.open("reboot_err.txt", "w")

# get command to exec a new process
if ARGV.empty? then
	$stderr.puts "Usage: ruby <this>.rb <CMD>..."
	exit false
end

# start
puts DateTime.now

# wait for parent (= before update) process to exit
# sleep 1 sec * WAIT_SEC times
puts "Waiting for parent process to exit..."
WAIT_SEC.times do
	break if Process.ppid == 1
	sleep 1
end
puts DateTime.now

# check ppid again
if Process.ppid != 1 then
	$stderr.puts "Timeout"
	exit false
end

# OK, I'll become a new (updated) process
puts "exec..."
puts ARGV
exec *ARGV
