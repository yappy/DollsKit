using CoreTweet;
using System.Diagnostics;
using System.IO;
using System.Xml.Serialization;

namespace Shanghai
{
    public class TwitterSettings
    {
        public static readonly string DefaultSetting = "please fill here";
        public bool WriteEnabled { get; set; } = false;
        public string ConsumerKey { get; set; } = DefaultSetting;
        public string ConsumerSecret { get; set; } = DefaultSetting;
        public string AccessToken { get; set; } = DefaultSetting;
        public string AccessSecret { get; set; } = DefaultSetting;
        public string MasterConsumerKey { get; set; } = DefaultSetting;
        public string MasterConsumerSecret { get; set; } = DefaultSetting;
        public string MasterAccessToken { get; set; } = DefaultSetting;
        public string MasterAccessSecret { get; set; } = DefaultSetting;
    }

    static class TwitterManager
    {
        private static readonly string SettingFileName = "twitter.xml";

        private static TwitterSettings settings;
        public static Tokens Tokens { get; set; }
        public static Tokens MasterTokens { get; set; }

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
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Twitter settings loaded");
            }
            if (!settings.WriteEnabled)
            {
                Log.Trace.TraceEvent(TraceEventType.Warning, 0,
                    "Twitter write feature is disabled. Only to log.");
            }

            Tokens = Tokens.Create(settings.ConsumerKey, settings.ConsumerSecret,
                settings.AccessToken, settings.AccessSecret);
            MasterTokens = Tokens.Create(settings.MasterConsumerKey, settings.MasterConsumerSecret,
                settings.MasterAccessToken, settings.MasterAccessSecret);
        }

        public static void Terminate()
        {
            settings = null;
            Tokens = null;
        }

        public static void Update(string msg, long? reply_to_status = null)
        {
            if (settings.WriteEnabled)
            {
                Tokens.Statuses.Update(status: msg, in_reply_to_status_id: reply_to_status);
            }
            else
            {
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Faked twitter update");
            }
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Twitter update: {0}", msg);
        }

        public static void Favorite(long id)
        {
            if (settings.WriteEnabled)
            {
                TwitterManager.Tokens.Favorites.Create(id);
            }
            else
            {
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Faked twitter favorite");
            }
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Twitter favorite: {0}", id);
        }
    }
}
