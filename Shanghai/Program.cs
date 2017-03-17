using System;
using System.Diagnostics;
using System.Configuration;

namespace Shanghai
{
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

            var flushLogTask = TaskParameter.Periodic("flushlog", toHour(1), toHour(1),
                (taskServer, taskName) =>
                {
                    Logger.Flush();
                });
            var bootMsgTask = TaskParameter.OneShot("boot", 0,
                (taskServer, taskName) =>
                {
                    TwitterManager.Update(string.Format("[{0}] {1}", DateTime.Now, bootMsg));
                });
            var healthCheckTask = TaskParameter.Periodic("health", 5, toHour(6),
                healthCheck.Check);

            var twitterCheckTask = TaskParameter.Periodic("twitter", 10, toMin(20),
                twitterCheck.CheckTwitter);

            var updateDdnsTask = TaskParameter.Periodic("ddns", 20, toHour(1),
                ddnsTask.UpdateTask);

            var cameraShotTask = TaskParameter.Periodic("camera", 30, toHour(4),
                cameraTask.TakePictureTask);
            var uploadPictureTask = TaskParameter.Periodic("uploadpic", 40, toMin(20),
                cameraTask.UploadPictureTask);

            return new TaskParameter[] {
                flushLogTask, bootMsgTask, healthCheckTask,
                twitterCheckTask, updateDdnsTask, cameraShotTask, uploadPictureTask,
            };
        }

        static void Main(string[] args)
        {
            Logger.AddConsole(LogLevel.Trace);
            Logger.AddFile(LogLevel.Info);
            Console.CancelKeyPress += (sender, eventArgs) => {
                Logger.Log(LogLevel.Info, "Interrupted");
                Logger.Flush();
            };

            Logger.Log(LogLevel.Info, "Start");

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

                        Logger.Log(LogLevel.Info, "Task server start");
                        ServerResult result = taskServer.Run(tasks);
                        Logger.Log(LogLevel.Info, "Task server exit");

                        bool exit;
                        switch (result)
                        {
                            case ServerResult.Reboot:
                                Logger.Log(LogLevel.Info, "Reboot");
                                exit = false;
                                break;
                            case ServerResult.Shutdown:
                                Logger.Log(LogLevel.Info, "Shutdown");
                                exit = true;
                                break;
                            case ServerResult.ErrorReboot:
                                errorRebootCount++;
                                Logger.Log(LogLevel.Info, "Reboot by Error ({0}/{1})", errorRebootCount, MaxErrorReboot);
                                exit = (errorRebootCount >= MaxErrorReboot);
                                break;
                            case ServerResult.FatalShutdown:
                                Logger.Log(LogLevel.Info, "Fatal Shutdown");
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
                    Logger.Log(LogLevel.Info, "GC...");
                    GC.Collect();
                    Logger.Log(LogLevel.Info, "GC complete");
                    ConfigurationManager.RefreshSection("AppSettings");
                    bootMsg = "Reboot...";
                }
            }
            catch (Exception e)
            {
                Logger.Log(LogLevel.Fatal, e);
            }

            Logger.Log(LogLevel.Info, "Terminate");
            Logger.Terminate();
        }
    }
}
