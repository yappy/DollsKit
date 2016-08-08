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
#if !DEBUG
        private static readonly int MaxTasks = 4;
        private static readonly int HeartBeatSec = 60;
#else
        private static readonly int MaxTasks = 4;
        private static readonly int HeartBeatSec = 3;
#endif
        private static readonly int MaxErrorReboot = 8;

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
#if true
            Func<int, int> toMin = (sec) => sec * 60;
            Func<int, int> toHour = (sec) => sec * 60 * 60;
            var healthCheck = new HealthCheck();
            var twitterCheck = new TwitterCheck();

            var healthCheckTask = TaskParameter.Periodic("health", 60, toHour(6),
                healthCheck.Check);

            var blackCheckTask = TaskParameter.Periodic("black", 1, toMin(10),
                twitterCheck.CheckBlack);
            var mentionCheckTask = TaskParameter.Periodic("mention", 2, toMin(2),
                twitterCheck.CheckMention);
            var updateIpAddrTask = TaskParameter.Periodic("ipaddr", 3, toHour(6),
                twitterCheck.updateIpAddr);

            return new TaskParameter[] { healthCheckTask,
                blackCheckTask, mentionCheckTask, updateIpAddrTask };
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
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Start");

            try
            {
                int errorRebootCount = 0;
                while (true)
                {
                    InitializeSystems();
                    {
                        var taskServer = new TaskServer(MaxTasks, HeartBeatSec);
                        TaskParameter[] tasks = SetupTasks();

                        Log.Trace.TraceEvent(TraceEventType.Information, 0, "Task server start");
                        ServerResult result = taskServer.Run(tasks);
                        Log.Trace.TraceEvent(TraceEventType.Information, 0, "Task server exit");

                        bool exit;
                        switch (result)
                        {
                            case ServerResult.Reboot:
                                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Reboot");
                                exit = false;
                                break;
                            case ServerResult.Shutdown:
                                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Shutdown");
                                exit = true;
                                break;
                            case ServerResult.ErrorReboot:
                                errorRebootCount++;
                                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Reboot by Error ({0}/{1})", errorRebootCount, MaxErrorReboot);
                                exit = (errorRebootCount >= MaxErrorReboot);
                                break;
                            case ServerResult.FatalShutdown:
                                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Fatal Shutdown");
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
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "GC...");
                    GC.Collect();
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "GC complete");
                }
            }
            catch (Exception e)
            {
                Log.Trace.TraceData(TraceEventType.Critical, 0, e);
            }

            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Terminate");
        }
    }
}
