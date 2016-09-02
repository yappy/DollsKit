using Accord.Neuro.Networks;
using CoreTweet;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Configuration;
using System.Diagnostics;

namespace Shanghai
{
    class TwitterCheck
    {
        // @ScreenName
        private static readonly string[] BlackList = {
            "Mewdra", "nippy2284", "metto0226", "CucumberDragon",
        };
        private static readonly int SettingMax = 100;
        private readonly ReadOnlyCollection<string> BlackWords, WhiteWords;
        private readonly ReadOnlyCollection<KeyValuePair<string, string>> ReplaceList;

        private DeepBeliefNetwork dlNetwork;

        public TwitterCheck()
        {
            var settings = ConfigurationManager.AppSettings;

            var blackWords = new List<string>();
            var whiteWords = new List<string>();
            for (int i = 1; i <= SettingMax; i++)
            {
                string black = settings["twitter.blackwords." + i];
                if (black == null) continue;
                foreach (var elem in black.Split(','))
                {
                    blackWords.Add(elem);
                }
                string white = settings["twitter.whitewords." + i];
                if (white == null) continue;
                foreach (var elem in white.Split(','))
                {
                    whiteWords.Add(elem);
                }
            }
            BlackWords = blackWords.AsReadOnly();
            WhiteWords = whiteWords.AsReadOnly();
            Log.Trace.TraceEvent(TraceEventType.Information, 0,
                "{0} black words loaded", BlackWords.Count);
            Log.Trace.TraceEvent(TraceEventType.Information, 0,
               "{0} white words loaded", WhiteWords.Count);

            var replaceList = new List<KeyValuePair<string, string>>();
            for (int i = 1; i <= SettingMax; i++)
            {
                string str = settings["twitter.replace." + i];
                if (str == null) continue;
                foreach (var pair in str.Split(','))
                {
                    string[] kv = pair.Split('=');
                    replaceList.Add(new KeyValuePair<string, string>(kv[0], kv[1]));
                }
            }
            ReplaceList = replaceList.AsReadOnly();
            Log.Trace.TraceEvent(TraceEventType.Information, 0,
                "{0} replace entries loaded", ReplaceList.Count);

            try
            {
                dlNetwork = DollsLib.Learning.DataManager.LoadDeepLearning(
                    SettingManager.Settings.Twitter.DlNetTrainError);
            }
            catch (Exception)
            {
                Log.Trace.TraceEvent(TraceEventType.Warning, 0,
                "DlNwtwork {0} load failed", SettingManager.Settings.Twitter.DlNetTrainError);
            }
        }

        /// <summary>
        /// 最先端のヒューリスティクスによるブラック判定
        /// </summary>
        /// <param name="status"></param>
        /// <param name="masterId"></param>
        /// <returns></returns>
        private bool IsBlack(Status status, long masterId)
        {
            // not master
            if (status.User.Id == masterId)
            {
                return false;
            }
            if (Array.IndexOf(BlackList, status.User.ScreenName) < 0)
            {
                return false;
            }

            string targetText = status.Text;
            foreach (var replace in ReplaceList)
            {
                targetText = targetText.Replace(replace.Key, replace.Value);
            }

            bool black = false;
            foreach (var word in BlackWords)
            {
                black = black || targetText.Contains(word);
            }

            const int AfterHour = 21;
            const int BeforeHour = 5;
            DateTimeOffset localTime = status.CreatedAt.ToLocalTime();
            black = black && (localTime.Hour >= AfterHour || localTime.Hour <= BeforeHour);

            return black;
        }

        private bool IsWhite(Status status, long masterId)
        {
            // master only
            if (status.User.Id != masterId)
            {
                return false;
            }

            bool white = false;
            foreach (var word in WhiteWords)
            {
                white = white || status.Text.Contains(word);
            }

            return white;
        }

        private void CheckMasterTimeline(string taskName)
        {
            const int SearchCount = 200;
            long masterId = TwitterManager.MasterTokens.Account.VerifyCredentials().Id ?? 0;

            var timeline = TwitterManager.MasterTokens.Statuses.HomeTimeline(count: SearchCount);

            // 判定器を使う
            if (dlNetwork != null)
            {
                var ln = new DollsLib.Learning.Learning();
                var workDataList = ln.CreateWorkDataList(timeline);
                var result = ln.Execute(dlNetwork, workDataList);
                for (int i = 0; i < result.Count; i++)
                {
                    if (result[i] > DollsLib.Learning.LearningCommon.Threshold)
                    {
                        var status = timeline[i];
                        Log.Trace.TraceEvent(TraceEventType.Information, 0,
                            "[{0}] Find by dl net {1:F3}: @{2} - {3}",
                            taskName, result[i], status.User.ScreenName, status.Text);
                        try
                        {
                            TwitterManager.Favorite(workDataList[i].Id);
                            TwitterManager.Update(
                                string.Format("@{0} ブラック #DollsLearning #試験中", status.User.ScreenName),
                                status.Id);
                        }
                        catch (TwitterException e)
                        {
                            Log.Trace.TraceEvent(TraceEventType.Error, 0, e.Message);
                        }
                    }
                }
            }
            // ヒューリスティクスを使う
            foreach (var status in timeline)
            {
                if (IsBlack(status, masterId))
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0,
                        "[{0}] Find black: @{1} - {2}", taskName, status.User.ScreenName, status.Text);
                    try
                    {
                        TwitterManager.Favorite(status.Id);
                        TwitterManager.Update(
                            string.Format("@{0} ブラック", status.User.ScreenName),
                            status.Id);
                    }
                    catch (TwitterException e)
                    {
                        Log.Trace.TraceEvent(TraceEventType.Error, 0, e.Message);
                    }
                }
                else if (IsWhite(status, masterId))
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0,
                        "[{0}] Find white: @{1} - {2}", taskName, status.User.ScreenName, status.Text);
                    try
                    {
                        TwitterManager.Favorite(status.Id);
                        TwitterManager.Update(
                            string.Format("@{0} ホワイト！", status.User.ScreenName),
                            status.Id);
                    }
                    catch (TwitterException e)
                    {
                        Log.Trace.TraceEvent(TraceEventType.Error, 0, e.Message);
                    }
                }
            }
        }

        private void CheckMentionTimeline(string taskName)
        {
            const int SearchCount = 200;
            long masterId = TwitterManager.MasterTokens.Account.VerifyCredentials().Id ?? 0;

            var timeline = TwitterManager.Tokens.Statuses.MentionsTimeline(count: SearchCount);
            foreach (var status in timeline)
            {
                if (status.User.Id != masterId && !(status.IsFavorited ?? false))
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0,
                        "[{0}] Find mention: @{1} - {2}", taskName, status.User.ScreenName, status.Text);
                    try
                    {
                        TwitterManager.Favorite(status.Id);
                        TwitterManager.Update(
                            string.Format("@{0} バカジャネーノ", status.User.ScreenName),
                            status.Id);
                    }
                    catch (TwitterException e)
                    {
                        Log.Trace.TraceEvent(TraceEventType.Error, 0, e.Message);
                    }
                }
            }
        }

        public void CheckTwitter(TaskServer server, string taskName)
        {
            CheckMasterTimeline(taskName);
            CheckMentionTimeline(taskName);
        }
    }
}
