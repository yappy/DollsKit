using System;
using System.Diagnostics;
using System.Threading;

namespace Shanghai
{
    static class Log
    {
        public static TraceSource Trace { get; private set; }
        static Log()
        {
            Trace = new TraceSource("TaskServer");
        }
    }

    class Program
    {
        static readonly int MaxErrorReboot = 8;

        static void InitializeSystems()
        {
            TwitterManager.Initialize();
        }

        static void TerminateSystems()
        {
            TwitterManager.Terminate();
        }

        static TaskParameter[] SetupTasks()
        {
#if !DEBUG
            Func<int, int> toHour = (sec) => sec * 60 * 60;
            var testTweetTask = TaskParameter.Periodic("tweet", 1, toHour(1),
                (TaskServer server, String taskName) =>
                {
                    TwitterManager.Update("Tweet test: " + DateTime.Now);
                });
            return new TaskParameter[] { testTweetTask };
#else
            var printTask = TaskParameter.Periodic("print", 0, 1,
                (TaskServer server, String taskName) =>
                {
                    Console.WriteLine("test");
                });
            var exitTask = TaskParameter.OneShot("shutdown", 5,
                (TaskServer server, String taskName) =>
                {
                    server.Shutdown(ServerResult.ErrorReboot);
                });
            var deadTestTask = TaskParameter.Periodic("takenoko", 0, 0,
                (TaskServer server, String taskName) =>
                {
                    Thread.Sleep(20 * 1000);
                });
            return new TaskParameter[] { printTask, exitTask, deadTestTask };
#endif
        }

        static void Main(string[] args)
        {
            Log.Trace.TraceInformation("Start");

            try
            {
                int errorRebootCount = 0;
                while (true)
                {
                    InitializeSystems();
                    {
                        var taskServer = new TaskServer();
                        TaskParameter[] tasks = SetupTasks();

                        Log.Trace.TraceInformation("Task server start");
                        ServerResult result = taskServer.Run(tasks);
                        Log.Trace.TraceInformation("Task server exit");

                        bool exit;
                        switch (result)
                        {
                            case ServerResult.Reboot:
                                Log.Trace.TraceInformation("Reboot");
                                exit = false;
                                break;
                            case ServerResult.Shutdown:
                                Log.Trace.TraceInformation("Shutdown");
                                exit = true;
                                break;
                            case ServerResult.ErrorReboot:
                                errorRebootCount++;
                                Log.Trace.TraceInformation("Reboot by Error ({0}/{1})", errorRebootCount, MaxErrorReboot);
                                exit = (errorRebootCount >= MaxErrorReboot);
                                break;
                            case ServerResult.FatalShutdown:
                                Log.Trace.TraceInformation("Fatal Shutdown");
                                exit = true;
                                break;
                            default:
                                Trace.Fail("must not reach");
                                exit = true;
                                break;
                        }
                        if (exit)
                        {
                            break;
                        }
                    }
                    TerminateSystems();
                    Log.Trace.TraceInformation("GC...");
                    GC.Collect();
                    Log.Trace.TraceInformation("GC complete");
                }
            }
            catch (Exception e)
            {
                Log.Trace.TraceData(TraceEventType.Critical, 0, e);
            }

            Log.Trace.TraceInformation("Terminate");
        }
    }
}
