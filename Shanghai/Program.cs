using System;
using System.Diagnostics;

namespace Shanghai
{
    class Program
    {
        private static readonly int MaxErrorReboot = 3;

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

        static void SetupTasks(TaskServer server, string bootMsg)
        {
            var healthCheck = new HealthCheck();
            var twitterCheck = new TwitterCheck();
            var ddnsTask = new DdnsTask();
            var cameraTask = new CameraTask();

            server.RegisterOneShotTask("bootmsg", TimeSpan.FromMinutes(0),
                (taskServer, taskName) =>
                {
                    TwitterManager.Update(bootMsg);
                });

            server.RegisterPeriodicTask("flushlog",
                (hour, min) => min == 55,
                (taskServer, taskName) =>
                {
                    Logger.Flush();
                });

            server.RegisterPeriodicTask("health",
                (hour, min) => (Array.IndexOf(new int[] { 0, 6, 12, 18 }, hour) >= 0) && (min == 0),
                healthCheck.Check);

            server.RegisterPeriodicTask("twitter",
                (hour, min) => Array.IndexOf(new int[] { 5, 25, 45 }, min) >= 0,
                twitterCheck.CheckTwitter);

            server.RegisterPeriodicTask("ddns",
                (hour, min) => min == 30,
                ddnsTask.UpdateTask);

            server.RegisterPeriodicTask("camera",
                (hour, min) => (Array.IndexOf(new int[] {  }, hour) >= 0) && (min == 0),
                cameraTask.TakePictureTask);
            server.RegisterPeriodicTask("uploadpic",
                (hour, min) => Array.IndexOf(new int[] { 10, 30, 50 }, min) >= 0,
                cameraTask.UploadPictureTask);
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
                        var taskServer = new TaskServer();
                        SetupTasks(taskServer, bootMsg);

                        Logger.Log(LogLevel.Info, "Task server start");
                        ServerResult result = taskServer.Run();
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
