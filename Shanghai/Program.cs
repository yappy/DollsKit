using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Text;

namespace Shanghai
{
    class Program
    {
        private static readonly int MaxErrorReboot = 3;
        private static readonly string RebootCmd = "ruby";
        private static readonly string RebootScript = "reboot.rb";

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

        static void SpawnRebootScript()
        {
            var asm = Assembly.GetEntryAssembly();
            if (asm == null)
            {
                throw new PlatformNotSupportedException("GetEntryAssembly failed");
            }
            string mainExePath = asm.Location;
            string cmd = "mono --debug " + Path.GetFileName(mainExePath);
            cmd = '"' + cmd + '"';

            var startInfo = new ProcessStartInfo();
            startInfo.FileName = RebootCmd;
            startInfo.Arguments = $"{RebootScript} {cmd}";
            startInfo.UseShellExecute = false;

            Logger.Log(LogLevel.Info, "Starting reboot script...");
            Logger.Log(LogLevel.Info, $"{RebootCmd} {RebootScript} {mainExePath}");

            using (var p = Process.Start(startInfo))
            {
                Logger.Log(LogLevel.Info,
                    $"Starting reboot script OK: pid = {p.Id}");
            }
        }

        static void Main(string[] args)
        {
            Logger.AddConsole(LogLevel.Trace);
            Logger.AddFile(LogLevel.Info);
            Console.CancelKeyPress += (sender, eventArgs) =>
            {
                Logger.Log(LogLevel.Info, "Interrupted");
                Logger.Flush();
            };

            Logger.Log(LogLevel.Info, "Start");

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

            try
            {
                int errorRebootCount = 0;
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
                            case ServerResult.Reboot:
                                Logger.Log(LogLevel.Info, "Reboot");
                                exit = false;
                                break;
                            case ServerResult.ErrorReboot:
                                errorRebootCount++;
                                Logger.Log(LogLevel.Info, "Reboot by Error ({0}/{1})", errorRebootCount, MaxErrorReboot);
                                exit = (errorRebootCount >= MaxErrorReboot);
                                break;
                            case ServerResult.Shutdown:
                                Logger.Log(LogLevel.Info, "Shutdown");
                                exit = true;
                                break;
                            case ServerResult.UpdateShutdown:
                                Logger.Log(LogLevel.Info, "Update Shutdown");
                                exit = true;
                                SpawnRebootScript();
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
                    bootMsg = "Reboot..." + gitInfo.ToString(); ;
                } // while (true)
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
