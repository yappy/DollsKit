using System.Diagnostics;
using System.IO;
using System.Xml.Serialization;

namespace Shanghai
{
    public class Settings
    {
        public TwitterSettings Twitter { get; set; } = new TwitterSettings();
        public DdnsSettings Ddns { get; set; } = new DdnsSettings();
    };

    public static class SettingManager
    {
        private static readonly string SettingFileName = "setting.xml";

        private static Settings settings;
        public static Settings Settings { get { return settings; } }

        public static void Initialize()
        {
            var xml = new XmlSerializer(typeof(Settings));
            if (!File.Exists(SettingFileName))
            {
                Log.Trace.TraceEvent(TraceEventType.Warning, 0,
                    "{0} not found. Create new.", SettingFileName);
                using (var stream = new FileStream(SettingFileName, FileMode.Create, FileAccess.Write))
                {
                    var def = new Settings();
                    xml.Serialize(stream, def);
                }
            }
            using (var stream = new FileStream(SettingFileName, FileMode.Open, FileAccess.Read))
            {
                settings = (Settings)xml.Deserialize(stream);
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Twitter settings loaded");
            }
        }

        public static void Terminate()
        {
            settings = null;
        }
    }
}
