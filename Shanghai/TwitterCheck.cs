using CoreTweet;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Shanghai
{
    class TwitterCheck
    {
        private static readonly string[] BlackList = {
            "Mewdra", "nippy", "metto0226",
        };

        public TwitterCheck()
        { }

        private bool IsBlack(Status status, long masterId)
        {
            // not master
            if (status.User.Id == masterId)
            {
                return false;
            }
            if (Array.IndexOf(BlackList, status.User.Name) < 0)
            {
                return false;
            }

            bool black = false;
            string[] Keywords = {
                "白", "黒", "ホワイト", "ブラック",
                "定時", "退社", "帰",
                "残業",
            };
            Array.ForEach(Keywords, (word) =>
            {
                black = black || status.Text.Contains(word);
            });

            const int AfterHour = 21;
            const int BeforeHour = 5;
            DateTimeOffset localTime = status.CreatedAt.ToLocalTime();
            black = black && (localTime.Hour >= AfterHour || localTime.Hour <= BeforeHour);

            return black;
        }

        private bool IsWhite(Status status)
        {
            // master only
            if (status.User.Id != TwitterManager.MasterTokens.UserId)
            {
                return false;
            }

            bool white = false;
            string[] Keywords = {
                "白", "ホワイト", "退社", "帰",
            };
            Array.ForEach(Keywords, (word) =>
            {
                white = white || status.Text.Contains(word);
            });

            return white;
        }

        public void CheckBlack(TaskServer server, string taskName)
        {
            const int SearchCount = 200;
            long masterId = TwitterManager.MasterTokens.Account.VerifyCredentials().Id ?? 0;

            var timeline = TwitterManager.MasterTokens.Statuses.HomeTimeline(count: SearchCount);
            foreach (var status in timeline)
            {
                if (IsBlack(status, masterId))
                {
                    if (!(status.IsFavorited ?? false))
                    {
                        Log.Trace.TraceEvent(TraceEventType.Information, 0,
                            "[{0}] Find black: {1} - {2}", taskName, status.User.Name, status.Text);
                        TwitterManager.Favorite(status.Id);
                        TwitterManager.Update(
                            string.Format("@{0} ブラック", status.User.Name),
                            status.Id);
                    }
                    else
                    {
                        Log.Trace.TraceEvent(TraceEventType.Information, 0, "Skip");
                    }
                }
            }
        }

        public void CheckMention(TaskServer server, string taskName)
        {
            const int SearchCount = 200;

            var timeline = TwitterManager.Tokens.Statuses.MentionsTimeline(count: SearchCount);
            foreach (var status in timeline)
            {
                if (!(status.IsFavorited ?? false))
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0,
                        "[{0}] Find mention: {1} - {2}", taskName, status.User.Name, status.Text);
                    TwitterManager.Favorite(status.Id);
                    TwitterManager.Update(
                        string.Format("@{0} バカジャネーノ", status.User.Name),
                        status.Id);
                }
            }
        }
    }
}
