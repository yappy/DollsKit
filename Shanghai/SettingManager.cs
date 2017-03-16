using Newtonsoft.Json;
using System.IO;

namespace Shanghai
{
    public class Settings
    {
        public TwitterSettings Twitter { get; set; }
        public DdnsSettings Ddns { get; set; }
        public CameraSettings Camera { get; set; }
        public WhiteSettings White { get; set; }
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
