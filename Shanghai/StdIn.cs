using System;
using System.Collections.Concurrent;
using System.Threading;
using System.Threading.Tasks;

namespace Shanghai
{
    static class StdIn
    {
        private static BlockingCollection<string> queue = new BlockingCollection<string>(1);

        // stdin から1行読んでは BlockingCollection に入れるスレッドを起動
        // プロセスの開始時に起動して終了まで放置する
        // stdin の非同期操作は問題が多そうなため、BlockingCollection 経由にする
        public static void Start()
        {
            Task.Run(()=>
            {
                string line;
                while ((line = Console.In.ReadLine()) != null)
                {
                    queue.Add(line);
                }
            });
        }

        public static string ReadLine(CancellationToken cancel)
        {
            return queue.Take(cancel);
        }
    }

    class StdInTask
    {
        // サーバキャンセル以外で終了しない one-shot task
        public void ProcessStdIn(TaskServer server, string taskName)
        {
            while (true)
            {
                // タスクサーバのキャンセルを見つつ1行読む
                string line = StdIn.ReadLine(server.CancelToken);
                Logger.Log(LogLevel.Info, $"[{taskName}] STDIN: {line}");

                switch (line)
                {
                    case "":
                        // ignore
                        break;
                    case "SHUTDOWN":
                        server.RequestShutdown(ServerResult.Shutdown);
                        break;
                    case "RELOAD":
                        server.RequestShutdown(ServerResult.Reload);
                        break;
                    default:
                        Logger.Log(LogLevel.Warning, $"[{taskName}] Unknown command: {line}");
                        break;
                }
            }
        }
    }
}
