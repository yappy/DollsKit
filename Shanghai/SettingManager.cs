using Newtonsoft.Json;
using System.IO;

namespace Shanghai
{
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
