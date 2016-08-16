using Accord.Neuro.Networks;
using CoreTweet;
using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Runtime.Serialization.Formatters.Binary;

namespace DollsLib.Learning
{
    /// <summary>
    /// 形態素解析済みの語句データ
    /// </summary>
    public class WordElement
    {
        /// <summary>
        /// 出現時の形
        /// </summary>
        public string Original { get; set; } = "";
        /// <summary>
        /// 原形 (辞書にない言葉の場合 "*")
        /// </summary>
        public string BaseForm { get; set; } = "";
        /// <summary>
        /// 品詞
        /// </summary>
        public string WordClass { get; set; } = "";

        public string GetBaseFormBestEffort()
        {
            return (BaseForm != "*") ? BaseForm : Original;
        }
    }

    /// <summary>
    /// 学習用に整形するための作業領域
    /// </summary>
    public class WorkDataEntry
    {
        /// <summary>
        /// tweet id
        /// </summary>
        public long Id { get; set; } = 0;
        public DateTime CreatedAt { get; set; } = DateTime.MinValue;
        public long UserId { get; set; } = 0;
        public string ScreenName { get; set; } = "";
        public string Text { get; set; } = "";
        public List<WordElement> AnalyzedText { get; set; } = new List<WordElement>();
        public string Teacher { get; set; } = "";
    }

    /// <summary>
    /// Twitter からのフルデータリスト
    /// tweet id で辞書アクセス可
    /// </summary>
    public class IdKeyedTweetList : KeyedCollection<long, Status>
    {
        protected override long GetKeyForItem(Status item)
        {
            return item.Id;
        }
    }

    /// <summary>
    /// 作業データのリスト
    /// tweet id で辞書アクセス可
    /// </summary>
    public class IdKeyedWorkDataList : KeyedCollection<long, WorkDataEntry>
    {
        protected override long GetKeyForItem(WorkDataEntry item)
        {
            return item.Id;
        }
    }

    public static class DataManager
    {
        public static readonly string RawDataFileName = "RawData.json";
        public static readonly string WorkDataFileName = "WorkData.json";
        public static readonly string EditDataFileName = "EditData.csv";
        public static readonly string BagDataFileName = "Bag.json";
        public static readonly string DeepLearningFileName = "Network{0}.bin";

        public static IdKeyedTweetList LoadRawData()
        {
            using (var reader = new StreamReader(RawDataFileName))
            {
                return JsonConvert.DeserializeObject<IdKeyedTweetList>(reader.ReadToEnd());
            }
        }

        public static void SaveRawData(IdKeyedTweetList list)
        {
            using (var writer = new StreamWriter(RawDataFileName))
            {
                writer.Write(JsonConvert.SerializeObject(list, Formatting.Indented));
            }
        }

        public static IdKeyedWorkDataList LoadWorkData()
        {
            using (var reader = new StreamReader(WorkDataFileName))
            {
                return JsonConvert.DeserializeObject<IdKeyedWorkDataList>(reader.ReadToEnd());
            }
        }

        public static List<WorkDataEntry> LoadTeacheredWorkData()
        {
            var workListOrg = LoadWorkData();
            var filtered = workListOrg.Where(entry => entry.Teacher != "");
            return new List<WorkDataEntry>(filtered);
        }

        public static void SaveWorkData(IdKeyedWorkDataList list)
        {
            using (var writer = new StreamWriter(WorkDataFileName))
            {
                writer.Write(JsonConvert.SerializeObject(list, Formatting.Indented));
            }
        }

        public static Dictionary<string, int> LoadBagOfWords()
        {
            using (var reader = new StreamReader(BagDataFileName))
            {
                return JsonConvert.DeserializeObject<Dictionary<string, int>>(reader.ReadToEnd());
            }
        }

        public static void SaveBagOfWords(Dictionary<string, int> bag)
        {
            using (var writer = new StreamWriter(BagDataFileName))
            {
                writer.Write(JsonConvert.SerializeObject(bag, Formatting.Indented));
            }
        }

        public static DeepBeliefNetwork LoadDeepLearning(int error)
        {
            string fileName = string.Format(DeepLearningFileName, error);
            using (var stream = new FileStream(fileName, FileMode.Open, FileAccess.Read))
            {
                var bf = new BinaryFormatter();
                return (DeepBeliefNetwork)bf.Deserialize(stream);
            }
        }

        public static void SaveDeepLearning(DeepBeliefNetwork dl, int error)
        {
            string fileName = string.Format(DeepLearningFileName, error);
            using (var stream = new FileStream(fileName, FileMode.Create, FileAccess.Write))
            {
                var bf = new BinaryFormatter();
                bf.Serialize(stream, dl);
            }
        }
    }
}
