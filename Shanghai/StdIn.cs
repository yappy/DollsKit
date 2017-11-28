using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace Shanghai
{
    static class StdIn
    {
        private static BlockingCollection<string> queue = new BlockingCollection<string>(1);

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
        // 終了しない one-shot task
        public void ProcessStdIn(TaskServer server, string taskName)
        {
            while (true)
            {
                // タスクサーバのキャンセルを見つつ1行読む
                string line = StdIn.ReadLine(server.CancelToken);
                Logger.Log(LogLevel.Info, $"[{taskName}] STDIN: {line}");

                switch (line)
                {
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
