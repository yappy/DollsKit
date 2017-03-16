using CoreTweet;
using System.IO;

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
                Logger.Log(LogLevel.Warning,
                    "Twitter write feature is disabled. Logging only.");
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
                Logger.Log(LogLevel.Info, "Faked twitter update ({0})", msg.Length);
                if (msg.Length > 140)
                {
                    Logger.Log(LogLevel.Info, "Message too long: {0}", msg.Length);
                }
            }
            Logger.Log(LogLevel.Info, "Twitter update: {0}", msg);
        }

        public static void UpdateWithImage(string msg, string imgPath)
        {
            if (settings.WriteEnabled)
            {
                MediaUploadResult media = Tokens.Media.Upload(
                    media: new FileInfo(imgPath));
                Logger.Log(LogLevel.Info, "Twitter upload: {0}", media.MediaId);
                Tokens.Statuses.Update(status: msg,
                    media_ids: new long[] { media.MediaId });
                Logger.Log(LogLevel.Info, "Twitter update with media: {0}", msg);
            }
            else
            {
                Update(msg);
            }
        }
    }
}
