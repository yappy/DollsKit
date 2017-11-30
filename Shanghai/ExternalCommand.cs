using System;
using System.Collections.Generic;
using System.Diagnostics;

namespace Shanghai
{
    static class ExternalCommand
    {
        public static List<string> Run(string cmd, string args,
            int timeoutSec, string workDir = ".")
        {
            var startInfo = new ProcessStartInfo();
            startInfo.FileName = cmd;
            startInfo.Arguments = args;
            startInfo.WorkingDirectory = workDir;
            startInfo.UseShellExecute = false;
            startInfo.RedirectStandardInput = true;
            startInfo.RedirectStandardOutput = true;
            startInfo.RedirectStandardError = true;

            Logger.Log(LogLevel.Info,
                "FileName = {0}, Args = {1}, WorkDir = {2}",
                startInfo.FileName, startInfo.Arguments, startInfo.WorkingDirectory);

            var output = new List<string>();
            using (var p = Process.Start(startInfo))
            {
                // stdin は即 EOF
                p.StandardInput.Close();

                p.OutputDataReceived += (sender, e) =>
                {
                    if (e.Data != null)
                    {
                        Logger.Log(LogLevel.Info, e.Data);
                        output.Add(e.Data);
                    }
                };
                p.ErrorDataReceived += (sender, e) =>
                {
                    if (e.Data != null)
                    {
                        Logger.Log(LogLevel.Warning, e.Data);
                    }
                };
                p.BeginOutputReadLine();
                p.BeginErrorReadLine();

                bool exited = p.WaitForExit(timeoutSec * 1000);
                if (!exited)
                {
                    p.Kill();
                    throw new TimeoutException();
                }
                // MS 実装も Mono 実装もタイムアウト付きの WaitForExit() では
                // (true を返したとしても) バッファが EOF まで読み出されたことは保証されない
                // 残りのデータすべてに対して DataReceived イベントハンドラを呼んでいる間に
                // タイムアウトした場合困るので仕方ないと思うがどこかにそう書いておいてほしい
                //
                // p.WaitForExit(timeout) が true を返した後なのでプロセスは終了しており
                // 出力イベントハンドラに null (EOF) が配送され終わるのだけを待つ
                p.WaitForExit();

                if (p.ExitCode != 0)
                {
                    throw new InvalidOperationException($"ExitCode={p.ExitCode}");
                }
                return output;
            }
        }

        public static List<string> RunNoThrow(string cmd, string args,
            int timeoutSec, string workDir = ".")
        {
            try
            {
                return Run(cmd, args, timeoutSec, workDir);
            }
            catch (Exception e)
            {
                Logger.Log(LogLevel.Info, e.Message);
                return new List<string>();
            }
        }

        public static string RunNoThrowOneLine(string cmd, string args,
            int timeoutSec, string workDir = ".")
        {
            List<string> lines = RunNoThrow(cmd, args, timeoutSec, workDir);
            return (lines.Count > 0) ? lines[0] : null;
        }
    }
}
