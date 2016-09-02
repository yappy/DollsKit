using CoreTweet;
using Newtonsoft.Json;
using System.IO;

namespace DollsLib.Twitter
{
    public class TwitterAccountSettings
    {
        public string ConsumerKey { get; set; }
        public string ConsumerSecret { get; set; }
        public string AccessToken { get; set; }
        public string AccessSecret { get; set; }
        public string MasterConsumerKey { get; set; }
        public string MasterConsumerSecret { get; set; }
        public string MasterAccessToken { get; set; }
        public string MasterAccessSecret { get; set; }
    }

    public class AccountManager
    {
        public static readonly string SettingFileName = "settings/TwitterAccount.json";

        public Tokens Tokens { get; set; }
        public Tokens MasterTokens { get; set; }

        public AccountManager()
        {
            TwitterAccountSettings settings;
            using (var reader = new StreamReader(SettingFileName))
            {
                settings = JsonConvert.DeserializeObject<TwitterAccountSettings>(
                    reader.ReadToEnd());
            }
            Tokens = Tokens.Create(settings.ConsumerKey, settings.ConsumerSecret,
                settings.AccessToken, settings.AccessSecret);
            MasterTokens = Tokens.Create(settings.MasterConsumerKey, settings.MasterConsumerSecret,
                settings.MasterAccessToken, settings.MasterAccessSecret);
        }
    }
}
