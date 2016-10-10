using System;
using System.Configuration;
using System.Diagnostics;

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
        private static readonly int MaxTasks = 4;
        private static readonly int HeartBeatSec = 60;
        private static readonly int MaxErrorReboot = 8;

        static void InitializeSystems()
        {
            SettingManager.Initialize();
            TwitterManager.Initialize();
        }

        static void TerminateSystems()
        {
            TwitterManager.Terminate();
            SettingManager.Terminate();
        }

        static TaskParameter[] SetupTasks(string bootMsg)
        {
            Func<int, int> toMin = (sec) => sec * 60;
            Func<int, int> toHour = (sec) => sec * 60 * 60;
            var healthCheck = new HealthCheck();
            var twitterCheck = new TwitterCheck();
            var ddnsTask = new DdnsTask();
            var cameraTask = new CameraTask();

            var bootMsgTask = TaskParameter.OneShot("boot", 0, (taskServer, taskName) =>
            {
                TwitterManager.Update(string.Format("[{0}] {1}", DateTime.Now, bootMsg));
            });
            var healthCheckTask = TaskParameter.Periodic("health", 5, toHour(6),
                healthCheck.Check);

            var twitterCheckTask = TaskParameter.Periodic("twitter", 10, toMin(10),
                twitterCheck.CheckTwitter);

            var updateDdnsTask = TaskParameter.Periodic("ddns", 20, toHour(1),
                ddnsTask.UpdateTask);

            var cameraShotTask = TaskParameter.Periodic("camera", /*30*/0, toHour(1),
                cameraTask.TakePictureTask);

            return new TaskParameter[] {
                bootMsgTask, healthCheckTask,
                twitterCheckTask, updateDdnsTask, cameraShotTask
            };
        }

        static void Main(string[] args)
        {
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Start");

            try
            {
                int errorRebootCount = 0;
                string bootMsg = "Boot...";
                while (true)
                {
                    InitializeSystems();
                    {
                        var taskServer = new TaskServer(MaxTasks, HeartBeatSec);
                        TaskParameter[] tasks = SetupTasks(bootMsg);

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
                    ConfigurationManager.RefreshSection("AppSettings");
                    bootMsg = "Reboot...";
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
