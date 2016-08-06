using CoreTweet;
using System.Diagnostics;
using System.IO;
using System.Xml.Serialization;

namespace Shanghai
{
    public class TwitterSettings
    {
        public static readonly string DefaultSetting = "please fill here";
        public bool Enabled { get; set; } = false;
        public string ConsumerKey { get; set; } = DefaultSetting;
        public string ConsumerSecret { get; set; } = DefaultSetting;
        public string AccessToken { get; set; } = DefaultSetting;
        public string AccessSecret { get; set; } = DefaultSetting;
    }

    static class TwitterManager
    {
        private static readonly string SettingFileName = "twitter.xml";

        private static TwitterSettings settings;
        private static Tokens tokens;

        public static void Initialize()
        {
            var xml = new XmlSerializer(typeof(TwitterSettings));
            if (!File.Exists(SettingFileName))
            {
                Log.Trace.TraceEvent(TraceEventType.Warning, 0,
                    "{0} not found. Create new.", SettingFileName);
                using (var stream = new FileStream(SettingFileName, FileMode.Create, FileAccess.Write))
                {
                    var def = new TwitterSettings();
                    xml.Serialize(stream, def);
                }
            }
            using (var stream = new FileStream(SettingFileName, FileMode.Open, FileAccess.Read))
            {
                settings = (TwitterSettings)xml.Deserialize(stream);
                Log.Trace.TraceInformation("Twitter settings loaded");
            }
            if (!settings.Enabled)
            {
                Log.Trace.TraceEvent(TraceEventType.Warning, 0,
                    "Twitter feature is disabled. Only to log.");
            }

            tokens = Tokens.Create(settings.ConsumerKey, settings.ConsumerSecret,
                settings.AccessToken, settings.AccessSecret);
        }

        public static void Terminate()
        {
            settings = null;
            tokens = null;
        }

        public static void Update(string msg)
        {
            if (settings.Enabled)
            {
                tokens.Statuses.Update(status: msg);
            }
            else
            {
                Log.Trace.TraceInformation("Faked twitter update");
            }
            Log.Trace.TraceInformation("Twitter update: {0}", msg);
        }
    }
}
