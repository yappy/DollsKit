using CoreTweet;
using System.Diagnostics;

namespace Shanghai
{
    public class TwitterSettings
    {
        public bool WriteEnabled { get; set; } = false;
        public int DlNetTrainError { get; set; } = 0;
    }

    static class TwitterManager
    {
        private static DollsLib.Twitter.AccountManager account;
        private static TwitterSettings settings;
        public static Tokens Tokens { get { return account.Tokens; } }
        public static Tokens MasterTokens { get { return account.MasterTokens; } }
        public static bool WriteEnabled { get { return settings.WriteEnabled; } }

        public static void Initialize()
        {
            account = new DollsLib.Twitter.AccountManager();
            settings = SettingManager.Settings.Twitter;

            if (!settings.WriteEnabled)
            {
                Log.Trace.TraceEvent(TraceEventType.Warning, 0,
                    "Twitter write feature is disabled. Only to log.");
            }
        }

        public static void Terminate()
        {
            account = null;
            settings = null;
        }

        public static void Update(string msg, long? reply_to_status = null)
        {
            if (settings.WriteEnabled)
            {
                Tokens.Statuses.Update(status: msg, in_reply_to_status_id: reply_to_status);
            }
            else
            {
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Faked twitter update ({0})", msg.Length);
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
