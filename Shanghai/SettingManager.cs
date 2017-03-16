using Newtonsoft.Json;
using System.Diagnostics;
using System.IO;

namespace Shanghai
{
    public class Settings
    {
        public TwitterSettings Twitter { get; set; } = new TwitterSettings();
        public DdnsSettings Ddns { get; set; } = new DdnsSettings();
        public CameraSettings camera { get; set; } = new CameraSettings();
    };

    public static class SettingManager
    {
        private static readonly string SettingFileName = "settings/ShanghaiOption.json";

        private static Settings settings;
        public static Settings Settings { get { return settings; } }

        public static void Initialize()
        {
            using (var reader = new StreamReader(SettingFileName))
            {
                settings = JsonConvert.DeserializeObject<Settings>(
                    reader.ReadToEnd());
                Logger.Log(LogLevel.Info, "Settings loaded");
            }
        }

        public static void Terminate()
        {
            settings = null;
        }
    }
}
