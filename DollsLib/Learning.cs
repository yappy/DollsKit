using Accord.Neuro;
using Accord.Neuro.Learning;
using Accord.Neuro.Networks;
using CoreTweet;
using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace DollsLib.Learning
{
    public class Learning
    {
        private class LearningOption
        {
            public static readonly string FileName = "settings/LearningOption.json";

            public List<Tuple<string, string>> ReplaceList { get; set; }
            public List<string> SkipWordClass { get; set; }
            public double CrossSplitRatio { get; set; }
            public List<int> HiddenNeurons { get; set; }
        }

        private LearningOption option;

        public Learning()
        {
            using (var reader = new StreamReader(LearningOption.FileName))
            {
                option = JsonConvert.DeserializeObject<LearningOption>(reader.ReadToEnd());
            }
        }

        public IdKeyedWorkDataList CreateWorkDataList(IEnumerable<Status> statusList)
        {
            var result = new IdKeyedWorkDataList();
            foreach (var status in statusList)
            {
                var workData = new WorkDataEntry();
                StatusToWorkData(workData, status);
                result.Add(workData);
            }
            // AnalyzedText をセット
            AnalyzeText(result);

            return result;
        }

        /// <summary>
        /// すべて同期
        /// Raw -> Work: 新しいツイートを追加
        /// Work 内の Text を形態素解析
        /// csv から教師データを取得
        /// WorkData ファイルの更新
        /// 新しい csv ファイルを書き出し
        /// </summary>
        public void UpdateWorkAndEditData()
        {
            // RawData と WorkData をロード
            // WorkData は存在しなければ空で開始
            IdKeyedTweetList rawList = DataManager.LoadRawData();
            IdKeyedWorkDataList workList;
            try
            {
                workList = DataManager.LoadWorkData();
            }
            catch (FileNotFoundException)
            {
                Console.WriteLine("File not found, create new.");
                workList = new IdKeyedWorkDataList();
            }
            // Raw->Work に必要なデータを移す
            // WorkDataEntry のメンバが増えた時のため、上書きしておく
            // (記入済みの teacher フィールド等は残す)
            // tweetid で探したエントリがなければ新規作成
            foreach (var raw in rawList)
            {
                WorkDataEntry workData;
                if (workList.Contains(raw.Id))
                {
                    workData = workList[raw.Id];
                    StatusToWorkData(workData, raw);
                }
                else
                {
                    workData = new WorkDataEntry();
                    StatusToWorkData(workData, raw);
                    workList.Add(workData);
                }
            }
            // AnalyzedText をセット
            AnalyzeText(workList);

            // (あれば) csv シートから WorkData に教師データを得る
            try
            {
                CsvToWorkData(workList);
            }
            catch (FileNotFoundException)
            {
                Console.WriteLine("csv file for edit not found");
            }
            // WorkDataList を保存
            DataManager.SaveWorkData(workList);
            // csv シートを更新
            WorkDataToCsv(workList);
        }

        private void StatusToWorkData(WorkDataEntry workData, Status status)
        {
            workData.Id = status.Id;
            workData.CreatedAt = status.CreatedAt.LocalDateTime;
            workData.UserId = status.User.Id ?? 0;
            workData.ScreenName = status.User.ScreenName;
            workData.Text = PreprocessText(status.Text);
            //workData.AnalyzedText
            //workData.Teacher
        }

        /// <summary>
        /// 改行などの空白文字はすべてスペースに
        /// カンマは csv のため除去
        /// 強制単純置換 ("*置換後*"でマークされます)
        /// </summary>
        /// <param name="text"></param>
        /// <returns></returns>
        private string PreprocessText(string text)
        {
            text = Regex.Replace(text, @"\s", " ");
            text = Regex.Replace(text, @"\\0", " ");
            text = text.Replace(',', ' ');
            foreach (var entry in option.ReplaceList)
            {
                text = text.Replace(entry.Item1, "*" + entry.Item2 + "*");
            }
            return text;
        }

        /// <summary>
        /// Text を形態素解析して AnalyzedText にセット
        /// </summary>
        private void AnalyzeText(IdKeyedWorkDataList workList)
        {
            const string InFile = "mecabin.txt";
            const string OutFile = "mecabout.txt";

            // 入力ファイル生成
            // 1行1文
            using (var writer = new StreamWriter(InFile))
            {
                foreach (var workData in workList)
                {
                    writer.WriteLine(workData.Text);
                }
            }

            // mecab INFILE -o OUTFILE
            var p = Process.Start("mecab", string.Format("{0} -o {1}", InFile, OutFile));
            p.WaitForExit();

            // LIST := (LINE+) "EOS\n"
            // LINE := 表層形\t品詞,品詞細分類1,品詞細分類2,品詞細分類3,活用型,活用形,原形,読み,発音\n
            var analyzed = new List<WordElement>();
            using (var reader = new StreamReader(OutFile))
            {
                foreach (var workData in workList)
                {
                    workData.AnalyzedText.Clear();
                    string line;
                    while ((line = reader.ReadLine()) != "EOS")
                    {
                        string[] byTab = line.Split('\t');
                        string[] byComma = byTab[1].Split(',');
                        var elem = new WordElement();
                        elem.Original = byTab[0];
                        elem.BaseForm = byComma[6];
                        elem.WordClass = byComma[0];
                        workData.AnalyzedText.Add(elem);
                    }
                }
            }
        }

        /// <summary>
        /// WorkData を教師データ記入用の csv に展開
        /// </summary>
        /// <param name="workList"></param>
        private void WorkDataToCsv(IdKeyedWorkDataList workList)
        {
            // UTF8 +BOM (for Excel)
            using (var writer = new StreamWriter(DataManager.EditDataFileName, false, Encoding.UTF8))
            {
                writer.WriteLine("{0},{1},{2},{3},{4}",
                    "TID", "NAME", "TIME", "BLACK", "TEXT");
                foreach (var workData in workList)
                {
                    writer.WriteLine("X {0},{1},X {2},{3},{4}",
                        workData.Id, workData.ScreenName,
                        workData.CreatedAt.ToLocalTime(),
                        workData.Teacher, workData.Text);
                }
            }
        }

        /// <summary>
        /// csv から 教師データを WorkData に取り込む
        /// </summary>
        /// <param name="workList"></param>
        private void CsvToWorkData(IdKeyedWorkDataList workList)
        {
            using (var reader = new StreamReader(DataManager.EditDataFileName))
            {
                string[] header = reader.ReadLine().Split(',');
                int idCol = Array.IndexOf(header, "TID");
                int blackCol = Array.IndexOf(header, "BLACK");

                string line;
                while ((line = reader.ReadLine()) != null)
                {
                    string[] tokens = line.Split(',');
                    long tid = long.Parse(tokens[idCol].Split(' ')[1]);
                    string teacher = tokens[blackCol];
                    workList[tid].Teacher = teacher;
                }
            }
        }

        private struct WordAppear : IComparable<WordAppear>
        {
            public int Count;
            public string Word;

            public int CompareTo(WordAppear other)
            {
                int first = this.Count - other.Count;
                if (first != 0)
                {
                    return first;
                }
                else
                {
                    return this.Word.CompareTo(other.Word);
                }
            }
        }

        /// <summary>
        /// 単語を集計後、フィルタして Bag-of-Words を作る
        /// </summary>
        public void BagOfWords()
        {
            List<WorkDataEntry> workList = DataManager.LoadTeacheredWorkData();

            // 単語の出現数を数える
            var wordToCount = new Dictionary<string, int>();
            foreach (var entry in workList)
            {
                foreach (var elem in entry.AnalyzedText)
                {
                    // 品詞フィルタ
                    if (option.SkipWordClass.IndexOf(elem.WordClass) >= 0)
                    {
                        continue;
                    }
                    string word = elem.GetBaseFormBestEffort();
                    if (wordToCount.ContainsKey(word))
                    {
                        wordToCount[word]++;
                    }
                    else
                    {
                        wordToCount.Add(word, 1);
                    }
                }
            }
            // (Count, Word) のリストに変換してソート
            var appearList = new List<WordAppear>();
            foreach (var entry in wordToCount)
            {
                appearList.Add(new WordAppear
                {
                    Count = entry.Value,
                    Word = entry.Key
                });
            }
            appearList.Sort();
            // print
            foreach (var app in appearList)
            {
                double rate = app.Count * 100.0 / appearList.Count;
                Console.WriteLine("{0,4} {1,5:#0.0}% {2}",
                    app.Count, rate, app.Word);
            }

            // 値を入力してフィルタ
            Console.Write("MinCountFilter: ");
            int minCount = int.Parse(Console.ReadLine());
            Console.Write("MaxCountFilter: ");
            int maxCount = int.Parse(Console.ReadLine());

            int id = 0;
            var bagOfWords = new Dictionary<string, int>();
            foreach (var app in appearList)
            {
                if (app.Count >= minCount && app.Count <= maxCount)
                {
                    bagOfWords.Add(app.Word, id);
                    id++;
                }
            }
            Console.WriteLine("Dimension: {0} (original={1})", bagOfWords.Count, appearList.Count);
            DataManager.SaveBagOfWords(bagOfWords);
        }

        private static int GetInputDimension(Dictionary<string, int> bag)
        {
            return 2 + bag.Count;
        }

        private static double[] GetFeatureVector(Dictionary<string, int> bag, WorkDataEntry data)
        {
            var result = new List<double>();

            // [0] hour
            result.Add(data.CreatedAt.ToLocalTime().Hour);
            // [1] 曜日
            result.Add((int)data.CreatedAt.ToLocalTime().DayOfWeek);
            // [残り] 出現単語
            var wordVec = new double[bag.Count];
            foreach (var word in data.AnalyzedText)
            {
                string str = word.GetBaseFormBestEffort();
                if (bag.ContainsKey(str))
                {
                    wordVec[bag[str]] = 1.0;
                }
            }
            result.AddRange(wordVec);
            return result.ToArray();
        }

        private static double CrossValidate(DeepBeliefNetwork network,
            List<WorkDataEntry> validationList, Dictionary<string, int> bag,
            string outFileName)
        {
            double mse = 0.0;
            using (var writer = new StreamWriter(outFileName))
            {
                foreach (var entry in validationList)
                {
                    double[] input = GetFeatureVector(bag, entry);
                    double[] output = network.Compute(input);
                    double teacher = (entry.Teacher == "o") ? 1.0 : 0.0;

                    double error = output[0] - teacher;
                    mse += error * error;

                    bool dollsAns = output[0] > 0.5;
                    bool mastersAns = entry.Teacher == "o";
                    string kind;
                    if (dollsAns && mastersAns)
                    {
                        kind = "OK(TP)";
                    }
                    else if(!dollsAns && !mastersAns)
                    {
                        kind = "OK(TN)";
                    }
                    else if (dollsAns && !mastersAns)
                    {
                        kind = "NG(FP)";
                    }
                    else
                    {
                        kind = "NG(FN)";
                    }

                    writer.WriteLine("{0} {1:F5}, {2} @{3} {4}",
                        kind, output[0], entry.CreatedAt.ToLocalTime(), entry.ScreenName, entry.Text);
                }
                writer.WriteLine("MSE={0:F5}", mse);
            }
            return mse;
        }

        /// <summary>
        /// 学習
        /// </summary>
        public void DeepLearning()
        {
            List<WorkDataEntry> workList = DataManager.LoadTeacheredWorkData();
            Dictionary<string, int> bag = DataManager.LoadBagOfWords();

            // ガバガバランダムで教師付きデータを分割
            Random rand = new Random();
            int validationDataCount = (int)(workList.Count * option.CrossSplitRatio);
            var validationList = new List<WorkDataEntry>(validationDataCount);
            while (validationDataCount > 0)
            {
                int index = rand.Next(workList.Count);
                validationList.Add(workList[index]);
                workList.RemoveAt(index);
                validationDataCount--;
            }

            var hiddenNeurons = new List<int>(option.HiddenNeurons);
            hiddenNeurons.Add(1);
            Console.WriteLine("HiddenNeurons: {0}", string.Join(",", hiddenNeurons));

            var network = new DeepBeliefNetwork(
                inputsCount: GetInputDimension(bag),
                hiddenNeurons: hiddenNeurons.ToArray());

            new GaussianWeights(network).Randomize();
            network.UpdateVisibleWeights();

            var teacher = new BackPropagationLearning(network);
            var inmat = new List<double[]>();
            var outmat = new List<double[]>();
            foreach (var entry in workList)
            {
                double[] tin = GetFeatureVector(bag, entry);
                double[] tout = new double[] { entry.Teacher == "o" ? 1.0 : 0.0 };

                inmat.Add(tin.ToArray());
                outmat.Add(tout);
            }

            const int ErrorStep = 5;
            double error;
            int minError = int.MaxValue;
            double bestGenError = Double.MaxValue;
            int bestNumber = 0;
            do
            {
                error = teacher.RunEpoch(inmat.ToArray(), outmat.ToArray());
                Console.WriteLine("error={0} (best={1})", error, bestNumber);
                int intError = ((int)error + ErrorStep - 1) / ErrorStep * ErrorStep;
                if (intError < minError)
                {
                    network.UpdateVisibleWeights();
                    DataManager.SaveDeepLearning(network, intError);
                    Console.WriteLine("Save! ({0})", intError);
                    double genError = CrossValidate(network, validationList, bag,
                        string.Format("Result{0}.txt", intError));
                    Console.WriteLine("Validate (error={0:F5})", genError);
                    if (genError< bestGenError)
                    {
                        bestGenError = genError;
                        bestNumber = intError;
                    }

                    minError = intError;
                }
            } while (error > 30.0);
            network.UpdateVisibleWeights();
        }

        public void Eval(int trainError)
        {
            DeepBeliefNetwork network = DataManager.LoadDeepLearning(trainError);
            IdKeyedWorkDataList workList = DataManager.LoadWorkData();
            Dictionary<string, int> bag = DataManager.LoadBagOfWords();

            using (var writer = new StreamWriter("result.txt"))
            {
                foreach (var entry in workList)
                {
                    if (entry.Teacher != "")
                    {
                        continue;
                    }
                    double[] input = GetFeatureVector(bag, entry);
                    var output = network.Compute(input);

                    bool ans = output[0] > 0.5;

                    writer.WriteLine("{0:F5}, ans={1}, @{2} {3}",
                        output[0], ans, entry.ScreenName, entry.Text);
                }
            }
        }

        public List<double> Execute(DeepBeliefNetwork network, IdKeyedWorkDataList workList)
        {
            var result = new List<double>();
            Dictionary<string, int> bag = DataManager.LoadBagOfWords();
            foreach (var entry in workList)
            {
                double[] input = GetFeatureVector(bag, entry);
                var output = network.Compute(input);
                result.Add(output[0]);
            }

            return result;
        }
    }
}
