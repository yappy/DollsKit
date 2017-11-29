#
# Dolls Daemon
#
# - Manage Shanghai.exe process
# - Handle signals
# - Do update process
#

require 'logger'
require 'open3'

EXEC_CMD = "mono --debug Shanghai.exe"
LOG_FILE = "dollsd.log".freeze
IN_BUF_SIZE = 64 * 1024

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
			:result   => nil,
			:stdin    => nil,
			:stdout   => nil,
			:stderr   => nil,
			:rest_out => nil,
		};
	end

	def run
		setup
		loop do
			exec_proc
			break if !main_loop
		end
	end

private
	def setup
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
		@child[:result] = :result_shutdown
		@child[:stdin] = stdin
		@child[:stdout] = stdout
		@child[:stderr] = stderr
	end

	def on_exit_proc
		exit_code = @child[:wait_thr].value
		$logger.info "Dolls process exit (code=#{exit_code})"

		# drain until EOF
		while process_input do end

		@child[:stdin].close
		@child[:stdout].close
		@child[:stderr].close

		@child.clear
	end

	def main_loop
		reboot = false
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

				send_cmd("RELOAD");

				@sig_hup = false
			end
			# process input stream from the child
			process_input
			# wait for process exit
			if @child[:wait_thr].join(1) then
				reboot = true if @child[:result] == :result_reboot
				on_exit_proc
				break
			end
		end
		reboot
	end

	def intr_safe
		loop do
			begin
				return yield
			rescue Errno::EINTR
				# retry
			end
		end
	end

	def send_cmd(cmd_line)
		$logger.info "send command: #{cmd_line}"
		cmd_line = "\n" + cmd_line + "\n"
		expected = cmd_line.bytesize
		begin
			actual = intr_safe { @child[:stdin].write_nonblock(cmd_line) }
			if actual != expected then
				raise "write_nonblock #{expected} returns #{actual}"
			end
			true
		rescue => err
			$logger.error "send command failed"
			$logger.error err
			false
		end
	end

	def recv_cmd(cmd_line)
		$logger.info "recv command: #{cmd_line}"
		case cmd_line
		when "SHUTDOWN" then
			@child[:result] = :result_shutdown
		when "REBOOT" then
			@child[:result] = :result_reboot
		else
			$logger.warn "unknown command: #{cmd_line}"
		end
	end

	def process_input
		result = intr_safe {
			select([@child[:stdout], @child[:stderr]], [], [], 0)
		}
		process_any = !(result.nil?)

		result ||= [[], [], []]
		rs = result[0]
		if rs.include?(@child[:stdout]) then
			begin
				buf = intr_safe { @child[:stdout].sysread(IN_BUF_SIZE) }
				*lines, last = buf.split("\n", -1)
				lines[0] = @child[:rest_out].to_s + lines[0]
				@child[:rest_out] = last
				lines.each {|line| recv_cmd(line) }
			rescue EOFError
			end
		end
		if rs.include?(@child[:stderr]) then
			begin
				buf = intr_safe { @child[:stderr].sysread(IN_BUF_SIZE) }
				$logger.warn "stderr data (#{buf.bytesize})"
				$logger.warn buf
			rescue EOFError
			end
		end
		process_any
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
	$logger.info "[dollsd START]"
	begin
		DollsDaemon.new.run
	rescue => err
		$logger.fatal err
	ensure
		$logger.info "[dollsd EXIT]"
	end
end

main
