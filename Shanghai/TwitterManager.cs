using CoreTweet;
using System.Diagnostics;
using System.IO;
using System.Xml.Serialization;

namespace Shanghai
{
    public class TwitterSettings
    {
        public static readonly string DefaultSetting = "FillHere";
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
        private static TwitterSettings settings;
        public static Tokens Tokens { get; set; }
        public static Tokens MasterTokens { get; set; }
        public static bool WriteEnabled
        {
            get { return settings.WriteEnabled; }
        }

        public static void Initialize()
        {
            settings = SettingManager.Settings.Twitter;

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
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Faked twitter update ({0}", msg.Length);
                if (msg.Length > 140)
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "Message too long: {0}", msg.Length);
                }
            }
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Twitter update: {0}", msg);
        }

        public static void Favorite(long id)
        {
            if (settings.WriteEnabled)
            {
                Tokens.Favorites.Create(id);
            }
            else
            {
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Faked twitter favorite");
            }
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Twitter favorite: {0}", id);
        }

        public static void UpdateProfileLocation(string location)
        {
            if (settings.WriteEnabled)
            {
                Tokens.Account.UpdateProfile(location: location);
            }
            else
            {
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Faked twitter profile update");
            }
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "Twitter profile location: {0}", location);
        }
    }
}
