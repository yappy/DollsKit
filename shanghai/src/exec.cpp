#include "exec.h"
#include "util.h"
#include <chrono>
#include <thread>
#include <unistd.h>
#include <sys/wait.h>

namespace shanghai {

namespace {

using namespace std::chrono_literals;

const int RD = 0;
const int WR = 1;

void PipeClose(int &pfd)
{
	if (pfd >= 0) {
		close(pfd);
		pfd = -1;
	}
}

}	// namespace

void PipeDeleter::operator()(int *fds)
{
	PipeClose(fds[0]);
	PipeClose(fds[1]);
	delete[] fds;
}

Pipe Process::CreatePipe()
{
	int fd[2];
	util::SysCall(pipe(fd));

	Pipe pipe(new int[2], PipeDeleter());
	pipe[0] = fd[0];
	pipe[1] = fd[1];
	return pipe;
}

Process::Process(const std::string &path,
	std::initializer_list<std::string> args) : m_exit(false)
{
	Pipe in = CreatePipe();
	Pipe out = CreatePipe();
	Pipe err = CreatePipe();

	pid_t pid = util::SysCall(fork());
	if (pid == 0) {
		// child process
		// close stdio fds and duplicate fds to them
		dup2(in[RD], 0);
		dup2(out[WR], 1);
		dup2(err[WR], 2);
		// close all fds in int[2]
		in.reset();
		out.reset();
		err.reset();
		// exec
		// TODO: error
		// argv の要素が const char * でないのは互換性のため: const_cast で回避する
		// https://stackoverflow.com/questions/190184/execv-and-const-ness
		size_t argvlen = args.size() + 2;
		auto argv = std::make_unique<char *[]>(argvlen);
		argv[0] = const_cast<char *>(path.c_str());
		int argc = 1;
		for (const std::string &arg : args) {
			argv[argc] = const_cast<char *>(arg.c_str());
			argc++;
		}
		argv[argc] = nullptr;
		int ret = execv(path.c_str(), argv.get());
		// ここに来たら失敗している
		std::quick_exit(1);
	}
	else {
		// parent process
		m_pid = pid;
		// close fds to be unused
		PipeClose(in[RD]);
		PipeClose(out[WR]);
		PipeClose(err[WR]);
		// move to field (close at destruct)
		m_in = std::move(in);
		m_out = std::move(out);
		m_err = std::move(err);

		// start drain thread
		auto drain_func = [](int fd, std::string &outbuf) {
			char buf[1024];
			ssize_t size;
			while ((size = read(fd, buf, sizeof(buf))) > 0) {
				outbuf.append(buf, size);
			}
		};
		m_outth = std::thread(drain_func, m_out[RD], std::ref(m_outbuf));
		m_errth = std::thread(drain_func, m_err[RD], std::ref(m_errbuf));
	}
}

Process::~Process()
{
	if (!m_exit) {
		Kill();
		waitpid(m_pid, nullptr, 0);
	}
	// デストラクタで close されるが、何かあっても pipe を close すれば
	// スレッドは終了するはずなので先に close する
	m_in.reset();
	m_out.reset();
	m_err.reset();
	// ここで固まってほしくない
	m_outth.join();
	m_errth.join();
}

void Process::Kill()
{
	if (m_exit) {
		throw std::logic_error("Already exit");
	}
	// 既にゾンビになっていると失敗するのでエラーは無視する
	kill(m_pid, SIGKILL);
}

// 負のタイムアウトは無制限
int Process::WaitForExit(int timeout_sec)
{
	if (m_exit) {
		throw std::logic_error("Already exit");
	}

	// あまりいい方法がなさそうなので (waitpid にシグナルで割り込むのは NG)
	// 100ms ごとにポーリングする
	auto timeout = std::chrono::seconds(timeout_sec);
	auto start = std::chrono::system_clock::now();
	int status = 0;
	while (1) {
		int ret = util::SysCall(waitpid(m_pid, &status, WNOHANG));
		if (ret > 0) {
			// wait OK
			break;
		}
		// ret == 0 (not exited)
		auto now = std::chrono::system_clock::now();
		if (timeout_sec >= 0 && now - start >= timeout) {
			throw ProcessError("Process wait timeout");
		}
		std::this_thread::sleep_for(100ms);
	}
	// ゾンビの回収完了
	m_exit = true;
	// 終了状態の変換
	if (WIFEXITED(status)) {
		// exit() or main return code
		return WEXITSTATUS(status);
	}
	else if (WIFSIGNALED(status)) {
		// シグナル死
		// -(シグナル番号) を返す
		return -static_cast<int>(WTERMSIG(status));
	}
	else {
		// よく分からないので 1 を返したとする
		return 1;
	}
}

void Process::InputAndClose(const std::string &data)
{
	int &fd = m_in[WR];
	if (fd < 0) {
		throw std::logic_error("stdin is already closed");
	}
	auto size = data.size();
	const char *p = data.c_str();
	while (size > 0) {
		ssize_t wsize;
		try {
			wsize = util::SysCall(write(m_in[WR], p, size));
		}
		catch (...) {
			PipeClose(fd);
			throw;
		}
		p += wsize;
		size -= wsize;
	}
	PipeClose(fd);
}

const std::string &Process::GetOut()
{
	if (!m_exit) {
		throw std::logic_error("Not exit yet");
	}
	return m_outbuf;
}

const std::string &Process::GetErr()
{
	if (!m_exit) {
		throw std::logic_error("Not exit yet");
	}
	return m_errbuf;
}

}	// namespace shanghai
