#
# Dolls Daemon
#
# - Manage Shanghai.exe process
# - Handle signals
# - Do update process
#

require 'logger'

LOG_FILE = "dollsd.log".freeze

$logger = nil
$args = {
	:daemon		=> nil,
	:pid_file	=> nil,
};

class DollsDaemon
	def initialize
		@sig_int
		@sig_term = false
		@sig_hup = false
	end

	def run
		setup
		main_loop
	end

private
	def setup
		$logger.info "[START]"
		# create child and parent process will exit 0
		# stdin, stdout, stderr to /dev/null
		Process.daemon(nochdir = true, noclose = nil)
		$logger.info "daemon OK"

		# write daemon pid (fails if not exists)
		open($args[:pid_file], IO::WRONLY | IO::TRUNC) do |f|
			f << Process.pid
		end
		# if succeeded, register at_exit to delete the file
		at_exit do
			File.delete($args[:pid_file])
		end
		$logger.info "write pid file OK"

		# set signal handlers
		# reload
		Signal.trap(:HUP) { @sig_hup = true }
		# kill (SIG_INT for non-daemon mode)
		Signal.trap(:INT) { @sig_int = true }
		Signal.trap(:TERM) { @sig_term = true }
		$logger.info "sigaction OK"
	end

	def main_loop
		loop do
			if @sig_int or @sig_term then
				$logger.info "[SIGTERM]"
				@sig_int = false
				@sig_term = false
				break
			end
			if @sig_hup then
				$logger.info "[SIGHUP]"
				@sig_hup = false
			end
			sleep 1
		end
	end
end

def parse_args
	ARGV.each do |arg|
		if (arg == "--daemon") then
			$args[:daemon] = true
		elsif (arg =~ /^--pid-file=(.*)$/) then
			$args[:pid_file] = $1;
		else
			puts "Invalid arg: #{arg}"
		end
	end
	$args.each do |k, v|
		v.freeze
	end
end

def main
	parse_args
	# log ready
	$logger = Logger.new(LOG_FILE)

	# after that, output to $logger instead of stderr (it will be /dev/null)
	begin
		DollsDaemon.new.run
	rescue => err
		$logger.fatal err
	ensure
		$logger.info "[END]"
	end
end

main
