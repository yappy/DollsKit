#
# Dolls Daemon
#
# - Manage Shanghai.exe process
# - Handle signals
# - Do update process
#

require 'logger'
require 'open3'

EXEC_CMD = "ruby -e 'p $stdin.gets'"
LOG_FILE = "dollsd.log".freeze

$logger = nil
$args = {
	:daemon		=> nil,
	:pid_file	=> nil,
};

class DollsDaemon
	def initialize
		# signal flags
		@sig_int = false
		@sig_term = false
		@sig_hup = false

		# child process
		@child = {
			:wait_thr => nil,
			:stdin    => nil,
			:stdout   => nil,
			:stderr   => nil,
		};
	end

	def run
		setup
		exec_proc
		main_loop
	end

private
	def setup
		$logger.info "[START]"
		# create child and parent process will exit 0
		# stdin, stdout, stderr to /dev/null
		if $args[:daemon] then
			Process.daemon(nochdir = true, noclose = nil)
			$logger.info "daemon OK"
		else
			$logger.info "daemon SKIP"
		end

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

	def exec_proc
		raise "assert" if @child[:wait_thr]

		stdin, stdout, stderr, wait_thr = *Open3::popen3(EXEC_CMD)
		@child[:wait_thr] = wait_thr
		@child[:stdin] = stdin
		@child[:stdout] = stdout
		@child[:stderr] = stderr
	end

	def on_exit_proc
		exit_code = @child[:wait_thr].value
		$logger.info "Dolls process exit (code=#{exit_code})"

		@child[:stdin].close
		@child[:stdout].close
		@child[:stderr].close

		@child.clear
	end

	def main_loop
		loop do
			# process signals
			if @sig_int or @sig_term then
				$logger.info "[SIGTERM]"

				send_cmd("SHUTDOWN");

				@sig_int = false
				@sig_term = false
			end
			if @sig_hup then
				$logger.info "[SIGHUP]"

				# TODO: reload

				@sig_hup = false
			end
			# wait for process exit
			if @child[:wait_thr].join(1) then
				on_exit_proc
				break
			end
		end
	end

	def send_cmd(cmd)
		$logger.info "send command: #{cmd}"
		cmd = "\n" + cmd + "\n"
		loop do
			begin
				expected = cmd.bytesize
				actual = @child[:stdin].write_nonblock(cmd)
				if actual != expected then
					raise "write_nonblock #{expected} returns #{actual}"
				end
				return true
			rescue Errno::EINTR
				# do again
			rescue => err
				$logger.error "send command failed"
				$logger.error err
				return false
			end
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
	raise "--pid-file needed" unless $args[:pid_file]

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
