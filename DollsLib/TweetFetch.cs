using CoreTweet;
using System;
using System.IO;
using System.Linq;

namespace DollsLib.Learning
{
    public class TweetFetch
    {
        /// <summary>
        /// Twitter からデータを取得し生データファイルに追加する
        /// </summary>
        /// <param name="screenName">@名</param>
        public static void FetchFromTwitter(string screenName)
        {
            var account = new DollsLib.Twitter.AccountManager();
            Tokens masterTokens = account.MasterTokens;

            // 旧データのロード (無ければ空で初期化)
            IdKeyedTweetList list;
            try
            {
                list = DataManager.LoadRawData();
            }
            catch (FileNotFoundException)
            {
                Console.WriteLine("File not found, create new.");
                list = new IdKeyedTweetList();
            }
            Console.WriteLine("Data count: {0}", list.Count);

            // 200 件ずつ取得して持っていない tweet id があれば追加
            long? maxId = null;
            int count = 0;
            while (true)
            {
                var timeLine = masterTokens.Statuses.UserTimeline(
                    screen_name: screenName, count: 200, max_id: maxId);
                if (timeLine.Count == 0)
                {
                    break;
                }
                foreach (Status status in timeLine)
                {
                    if (!list.Contains(status.Id))
                    {
                        list.Add(status);
                    }
                    maxId = Math.Min(maxId ?? long.MaxValue, status.Id - 1);
                }
                count += timeLine.Count;

                Console.WriteLine("{0}...", count);
                Console.WriteLine("{0}", timeLine[0].Text);
            }
            // 降順にソート
            list.OrderByDescending((item) => item.Id);
            // 保存
            DataManager.SaveRawData(list);
            Console.WriteLine("Save OK");
            Console.WriteLine("Data count: {0}", list.Count);
        }
    }
}
