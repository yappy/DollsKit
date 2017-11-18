using Newtonsoft.Json;
using System;
using System.IO;

namespace Shanghai
{
    public class CommandLineSettings
    {
        public bool Help { get; set; } = false;
        public bool Daemon { get; set; } = false;
    }

    public class CommandLine
    {
        public static CommandLineSettings Settings { get; private set; }

        public static void Parse(string[] args)
        {
            Settings = new CommandLineSettings();
            foreach (var arg in args)
            {
                int idx = arg.IndexOf('=');
                string left = (idx >= 0) ? arg.Substring(0, idx) : arg;
                string right = (idx >= 0) ? arg.Substring(idx + 1) : "";
                switch (left)
                {
                    case "--help":
                        Settings.Help = true;
                        break;
                    case "--daemon":
                        Settings.Daemon = true;
                        break;
                    default:
                        throw new ArgumentException($"Unknown param: {left}");
                }
            }
        }

        public static void PrintHelp()
        {
            Console.Error.WriteLine("Usage");
            Console.Error.WriteLine("--help");
            Console.Error.WriteLine("    Print this help.");
            Console.Error.WriteLine("--daemon");
            Console.Error.WriteLine("    Disable stdout.");
        }
    }

    public class Settings
    {
        public DatabaseSettings Database { get; set; }
        public TwitterSettings Twitter { get; set; }
        public DdnsSettings Ddns { get; set; }
        public CameraSettings Camera { get; set; }
        public WhiteSettings White { get; set; }
    };

    public static class SettingManager
    {
        private static readonly string SettingFileName = "settings/ShanghaiOption.json";

        public static Settings Settings { get; private set; }

        public static void Initialize()
        {
            using (var reader = new StreamReader(SettingFileName))
            {
                Settings = JsonConvert.DeserializeObject<Settings>(
                    reader.ReadToEnd());
                Logger.Log(LogLevel.Info, "Settings loaded");
            }
        }

        public static void Terminate()
        {
            Settings = null;
        }
    }
}
