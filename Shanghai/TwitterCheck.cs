using Accord.Neuro.Networks;
using CoreTweet;
using DollsLang;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Web;

namespace Shanghai
{
    public class WhiteSettings
    {
        public List<string> BlackList { get; set; } = new List<string>();
        public List<string> BlackWords { get; set; } = new List<string>();
        public List<string> WhiteWords { get; set; } = new List<string>();
        public Dictionary<string, string> ReplaceList { get; set; } = new Dictionary<string, string>();
    }

    class TwitterCheck
    {
        private WhiteSettings Setting;
        private long? SinceId = null;

        private DeepBeliefNetwork DlNetwork;

        public TwitterCheck()
        {
            Setting = SettingManager.Settings.White;
            
            Logger.Log(LogLevel.Info,
                "{0} black list loaded", Setting.BlackList.Count);
            Logger.Log(LogLevel.Trace,
                string.Join(",", Setting.BlackList));
            Logger.Log(LogLevel.Info,
                "{0} black words loaded", Setting.BlackWords.Count);
            Logger.Log(LogLevel.Trace,
                string.Join(",", Setting.BlackWords));
            Logger.Log(LogLevel.Info,
                "{0} white words loaded", Setting.WhiteWords.Count);
            Logger.Log(LogLevel.Trace,
                string.Join(",", Setting.WhiteWords));
            Logger.Log(LogLevel.Info,
               "{0} replace list loaded", Setting.ReplaceList.Count);
            Logger.Log(LogLevel.Trace, Setting.ReplaceList.Aggregate(new StringBuilder(),
                (sb, kvp) => sb.AppendFormat(" {0}={1}", kvp.Key, kvp.Value)).ToString());

            try
            {
                DlNetwork = DollsLib.Learning.DataManager.LoadDeepLearning(
                    SettingManager.Settings.Twitter.DlNetTrainError);
            }
            catch (Exception)
            {
                Logger.Log(LogLevel.Info,
                    "DlNwtwork {0} load failed",
                    SettingManager.Settings.Twitter.DlNetTrainError);
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
            // BlackList filter
            if (!Setting.BlackList.Contains(status.User.ScreenName))
            {
                return false;
            }

            string targetText = status.Text;
            foreach (var replace in Setting.ReplaceList)
            {
                targetText = targetText.Replace(replace.Key, replace.Value);
            }

            bool black = false;
            foreach (var word in Setting.BlackWords)
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
            foreach (var word in Setting.WhiteWords)
            {
                white = white || status.Text.Contains(word);
            }

            return white;
        }

        private bool IsProgram(Status status)
        {
            return status.Entities.HashTags.Any((ent) => ent.Text == "人形語");
        }

        private string ExecuteProgram(TaskServer server, string src, out Bitmap graphicsResult)
        {
            var lexer = new Lexer();
            var parser = new Parser();
            var timeoutSource = new CancellationTokenSource();
            var linkedSource = CancellationTokenSource.CreateLinkedTokenSource(
                server.CancelToken, timeoutSource.Token);
            var runtime = new Runtime(linkedSource.Token);
            string output = "";
            graphicsResult = null;
            try
            {
                var tokenList = lexer.Process(src);
                var program = parser.Parse(tokenList);

                runtime.LoadDefaultLibrary();
                // TODO: timeout
                timeoutSource.CancelAfter(1000);
                runtime.Execute(program, out output, out graphicsResult);
            }
            catch (LangException e)
            {
                // 通常の言語処理系エラー
                output = e.Message;
            }
            catch (OperationCanceledException)
            {
                if (timeoutSource.IsCancellationRequested)
                {
                    // 実行タイムアウト
                    output = "Time Limit Exceeded";
                }
                else
                {
                    // サーバに対するキャンセルは外へリスロー
                    throw;
                }
            }
            catch(Exception e)
            {
                // 起きてはいけないはずのエラー
                // FatalLangException
                // またはライブラリから例外がスローされてしまった
                output = "Language system fatal error (管理者へ連絡してください)";
                Logger.Log(LogLevel.Error, e);
            }
            return output;
        }

        private long CheckMasterTimeline(TaskServer server, string taskName)
        {
            const int SearchCount = 200;
            long selfId = TwitterManager.Tokens.Account.VerifyCredentials().Id ?? 0;
            long masterId = TwitterManager.MasterTokens.Account.VerifyCredentials().Id ?? 0;

            long nextSinceId = 0;
            var timeline = TwitterManager.MasterTokens.Statuses.HomeTimeline(
                count: SearchCount, since_id: SinceId);

            // プログラム判定
            foreach (var status in timeline)
            {
                // 自分のツイートには反応しない
                if (status.User.Id == selfId)
                {
                    continue;
                }
                // リツイートは除外
                if (status.RetweetedStatus != null)
                {
                    continue;
                }
                if (IsProgram(status))
                {
                    string src = HttpUtility.HtmlDecode(status.Text);
                    Logger.Log(LogLevel.Info, "[{0}] Find program: @{1} - {2}",
                        taskName, status.User.ScreenName, src);

                    Bitmap graphicsResult;
                    string output = ExecuteProgram(server, src, out graphicsResult);
                    // @ と # は禁止する
                    output = output.Replace('@', ' ');
                    output = output.Replace('#', ' ');
                    // 全角も禁止する
                    output = output.Replace('＠', ' ');
                    output = output.Replace('＃', ' ');

                    // リプライ先を付与した後文字数制限で切り詰める
                    string tweet = string.Format("@{0}\n{1}", status.User.ScreenName, output);
                    // TODO max_tweet
                    tweet = tweet.Substring(0, Math.Min(tweet.Length, 140));
                    try
                    {
                        if (graphicsResult == null)
                        {
                            // 文字だけツイート
                            TwitterManager.Update(tweet, status.Id);
                        }
                        else
                        {
                            // 画像付きツイート
                            string tmpPath = Path.GetTempPath() + Guid.NewGuid() + ".png";
                            Logger.Log(LogLevel.Info, "Save a result image to: {0}", tmpPath);
                            graphicsResult.Save(tmpPath, ImageFormat.Png);
                            try
                            {
                                TwitterManager.UpdateWithImage(tweet, tmpPath);
                            }
                            finally
                            {
                                try
                                {
                                    File.Delete(tmpPath);
                                }
                                catch (Exception e)
                                {
                                    Logger.Log(LogLevel.Warning, e);
                                }
                            }
                        }
                        nextSinceId = Math.Max(status.Id, nextSinceId);
                    }
                    catch (TwitterException e)
                    {
                        Logger.Log(LogLevel.Error, e.Message);
                    }
                }
            }

            // 判定器を使う
            if (DlNetwork != null)
            {
                var ln = new DollsLib.Learning.Learning();
                var workDataList = ln.CreateWorkDataList(timeline);
                var result = ln.Execute(DlNetwork, workDataList);
                for (int i = 0; i < result.Count; i++)
                {
                    var status = timeline[i];
                    // Black List filter
                    if (!Setting.BlackList.Contains(status.User.ScreenName))
                    {
                        continue;
                    }
                    // リツイートは除外
                    if (status.RetweetedStatus != null)
                    {
                        continue;
                    }
                    if (result[i] > DollsLib.Learning.LearningCommon.Threshold)
                    {
                        Logger.Log(LogLevel.Info,
                            "[{0}] Find by dl net {1:F3}: @{2} - {3}",
                            taskName, result[i], status.User.ScreenName, status.Text);
                        try
                        {
                            TwitterManager.Update(
                                string.Format("@{0} ブラック #DollsLearning #試験中", status.User.ScreenName),
                                status.Id);
                            nextSinceId = Math.Max(status.Id, nextSinceId);
                        }
                        catch (TwitterException e)
                        {
                            Logger.Log(LogLevel.Error, e.Message);
                        }
                    }
                }
            }
            // ヒューリスティクスを使う
            foreach (var status in timeline)
            {
                // リツイートは除外
                if (status.RetweetedStatus != null)
                {
                    continue;
                }
                if (IsBlack(status, masterId))
                {
                    Logger.Log(LogLevel.Info, "[{0}] Find black: @{1} - {2}",
                        taskName, status.User.ScreenName, status.Text);
                    try
                    {
                        TwitterManager.Update(
                            string.Format("@{0} ブラック", status.User.ScreenName),
                            status.Id);
                        nextSinceId = Math.Max(status.Id, nextSinceId);
                    }
                    catch (TwitterException e)
                    {
                        Logger.Log(LogLevel.Error, e.Message);
                    }
                }
                else if (IsWhite(status, masterId))
                {
                    Logger.Log(LogLevel.Info, "[{0}] Find white: @{1} - {2}",
                        taskName, status.User.ScreenName, status.Text);
                    try
                    {
                        TwitterManager.Update(
                            string.Format("@{0} ホワイト！", status.User.ScreenName),
                            status.Id);
                        nextSinceId = Math.Max(status.Id, nextSinceId);
                    }
                    catch (TwitterException e)
                    {
                        Logger.Log(LogLevel.Error, e.Message);
                    }
                }
            }
            return nextSinceId;
        }

        private long CheckMentionTimeline(string taskName)
        {
            const int SearchCount = 200;
            long selfId = TwitterManager.Tokens.Account.VerifyCredentials().Id ?? 0;
            long masterId = TwitterManager.MasterTokens.Account.VerifyCredentials().Id ?? 0;

            long nextSinceId = 0;
            var timeline = TwitterManager.Tokens.Statuses.MentionsTimeline(
                count: SearchCount, since_id: SinceId);

            foreach (var status in timeline)
            {
                if (status.User.Id == selfId || status.User.Id == masterId)
                {
                    continue;
                }
                Logger.Log(LogLevel.Info, "[{0}] Find mention: @{1} - {2}",
                    taskName, status.User.ScreenName, status.Text);
                try
                {
                    TwitterManager.Update(
                        string.Format("@{0} バカジャネーノ", status.User.ScreenName),
                        status.Id);
                    nextSinceId = Math.Max(status.Id, nextSinceId);
                }
                catch (TwitterException e)
                {
                    Logger.Log(LogLevel.Error, e.Message);
                }
            }
            return nextSinceId;
        }

        private void SetInitialSinceId()
        {
            const int SearchCount = 200;
            var timeline = TwitterManager.Tokens.Statuses.UserTimeline(
                count: SearchCount, since_id: SinceId, exclude_replies: false, include_rts: false);
            foreach (var status in timeline)
            {
                SinceId = Math.Max(SinceId ?? 0, status.Id);
            }
        }

        public void CheckTwitter(TaskServer server, string taskName)
        {
            // 最初は自分の最後のツイートIDにセット
            if (SinceId == null)
            {
                SetInitialSinceId();
                Logger.Log(LogLevel.Info, "SinceId: {0}", SinceId);
            }
            // リプライしたIDの最大値を次の sinceId とする
            long nextSinceId = 0;
            nextSinceId = Math.Max(CheckMasterTimeline(server, taskName), nextSinceId);
            nextSinceId = Math.Max(CheckMentionTimeline(taskName), nextSinceId);
            if (nextSinceId > 0)
            {
                SinceId = nextSinceId;
            }
        }
    }
}
