using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Text;

namespace Shanghai
{
    class Program
    {
        static readonly int ErrorReloadLimit = 5;

        static void InitializeSystems()
        {
            SettingManager.Initialize();
            DatabaseManager.Initialize();
            TwitterManager.Initialize();
        }

        static void TerminateSystems()
        {
            TwitterManager.Terminate();
            DatabaseManager.Terminate();
            SettingManager.Terminate();
        }

        // Called after InitializeSystems() statically.
        // For new task test.
        static void TaskTest(TaskServer server)
        {
            // Stub
#if DEBUG
            Logger.Log(LogLevel.Info, "Task Test");
            new UpdateCheck().Check(server, "update");
#endif
        }

        static void SetupTasks(TaskServer server, string bootMsg)
        {
            var stdInTask = new StdInTask();
            var healthCheck = new HealthCheck();
            var twitterCheck = new TwitterCheck();
            var ddnsTask = new DdnsTask();
            var cameraTask = new CameraTask();
            var updateCheck = new UpdateCheck();

            server.RegisterOneShotTask("bootmsg", TimeSpan.FromMinutes(0),
                (taskServer, taskName) =>
                {
                    TwitterManager.Update(string.Format("[{0}] {1}", DateTime.Now, bootMsg));
                });
            server.RegisterOneShotTask("stdin", TimeSpan.FromMinutes(0), stdInTask.ProcessStdIn);

            server.RegisterPeriodicTask("flushlog",
                (hour, min) => min == 55,
                (taskServer, taskName) =>
                {
                    Logger.Flush();
                });

            server.RegisterPeriodicTask("health",
                (hour, min) => (Array.IndexOf(new int[] { 0, 6, 12, 18 }, hour) >= 0) && (min == 59),
                healthCheck.Check);

            server.RegisterPeriodicTask("twitter",
                (hour, min) => Array.IndexOf(new int[] { 0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55 }, min) >= 0,
                twitterCheck.CheckTwitter);

            server.RegisterPeriodicTask("ddns",
                (hour, min) => min == 30,
                ddnsTask.UpdateTask);

            server.RegisterPeriodicTask("camera",
                (hour, min) => (Array.IndexOf(new int[] { 0, 6, 12, 18 }, hour) >= 0) && (min == 0),
                cameraTask.TakePictureTask);
            server.RegisterPeriodicTask("uploadpic",
                (hour, min) => Array.IndexOf(new int[] { 10, 30, 50 }, min) >= 0,
                cameraTask.UploadPictureTask);

            server.RegisterPeriodicTask("update",
                (hour, min) => Array.IndexOf(new int[] { 5, 15, 25, 35, 45, 55 }, min) >= 0,
                updateCheck.Check);
        }

        static void Main(string[] args)
        {
            try
            {
                CommandLine.Parse(args);
            }
            catch (ArgumentException e)
            {
                Console.WriteLine(e.Message);
                return;
            }
            if (CommandLine.Settings.Help)
            {
                CommandLine.PrintHelp();
                return;
            }

            if (!CommandLine.Settings.Daemon)
            {
                Logger.AddConsole(LogLevel.Trace);
            }
            Logger.AddFile(LogLevel.Info);

            // Start stdin task
            StdIn.Start();

            Logger.Log(LogLevel.Info, "Start");

            string cmdToDaemon = null;
            try
            {
                // Get git info
                var gitInfo = new StringBuilder();
                {
                    string str = ExternalCommand.RunNoThrowOneLine(
                        "git", "rev-parse --abbrev-ref HEAD", 1);
                    gitInfo.Append((str != null) ? ('\n' + str) : "");
                }
                {
                    string str = ExternalCommand.RunNoThrowOneLine(
                        "git", "rev-parse HEAD", 1);
                    gitInfo.Append((str != null) ? (' ' + str) : "");
                }

                int errorCount = 0;
                string bootMsg = "Boot..." + gitInfo.ToString();
                while (true)
                {
                    InitializeSystems();
                    {
                        var taskServer = new TaskServer();
                        SetupTasks(taskServer, bootMsg);
                        TaskTest(taskServer);

                        Logger.Log(LogLevel.Info, "Task server start");
                        ServerResult result = taskServer.Run();
                        Logger.Log(LogLevel.Info, "Task server exit");

                        bool exit;
                        switch (result)
                        {
                            case ServerResult.Reload:
                                Logger.Log(LogLevel.Info, "Reload");
                                errorCount = 0;
                                bootMsg = "Reload..." + gitInfo.ToString();
                                exit = false;
                                break;
                            case ServerResult.ErrorReload:
                                Logger.Log(LogLevel.Info, "Error Reboot");
                                errorCount++;
                                bootMsg = $"Reload... (Error {errorCount}/{ErrorReloadLimit})" + gitInfo.ToString();
                                exit = false;
                                break;
                            case ServerResult.Shutdown:
                                Logger.Log(LogLevel.Info, "Shutdown");
                                exit = true;
                                break;
                            case ServerResult.UpdateReboot:
                                Logger.Log(LogLevel.Info, "Update Reboot");
                                cmdToDaemon = "REBOOT";
                                exit = true;
                                break;
                            case ServerResult.FatalShutdown:
                                Logger.Log(LogLevel.Info, "Fatal Shutdown");
                                exit = true;
                                break;
                            default:
                                Logger.Log(LogLevel.Fatal, "must not reach");
                                exit = true;
                                break;
                        }
                        if (exit)
                        {
                            // shutdown/reboot
                            break;
                        }
                    }
                    // reload
                    TerminateSystems();
                    Logger.Log(LogLevel.Info, "GC...");
                    GC.Collect();
                    Logger.Log(LogLevel.Info, "GC complete");
                } // while (true)
            }
            catch (Exception e)
            {
                Logger.Log(LogLevel.Fatal, e);
            }
            finally
            {
                if (cmdToDaemon != null)
                {
                    Console.WriteLine(cmdToDaemon);
                }
                Logger.Log(LogLevel.Info, "Terminate");
                Logger.Terminate();
            }
        }
    }
}
