using System;

namespace DLearn
{
    class Program
    {
        private static void Usage()
        {
            Console.WriteLine("fetch <ScreenName>");
            Console.WriteLine("\tUpdate RawData");
            Console.WriteLine("update");
            Console.WriteLine("\tSync Raw-Work-CVS Data");
            Console.WriteLine("bag");
            Console.WriteLine("\tCreate Bag-of-words");
            Console.WriteLine("dl");
            Console.WriteLine("\tDeep Learning");
            Console.WriteLine("eval <TrainError>");
            Console.WriteLine("\tEvaluate by tweet data in WorkData");
        }

        static void Main(string[] args)
        {
            try
            {
                Usage();
                if (args.Length == 0)
                {
                    Console.Write("> ");
                    args = Console.ReadLine().Split(' ');
                }
                var learning = new DollsLib.Learning.Learning();
                switch (args[0])
                {
                    case "fetch":
                        DollsLib.Learning.TweetFetch.FetchFromTwitter(args[1]);
                        break;
                    case "update":
                        learning.UpdateWorkAndEditData();
                        break;
                    case "bag":
                        learning.BagOfWords();
                        break;
                    case "dl":
                        learning.DeepLearning();
                        break;
                    case "eval":
                        learning.Eval(int.Parse(args[1]));
                        break;
                    default:
                        Console.WriteLine("Invalid command");
                        break;
                }
            }
            catch (Exception e)
            {
                Console.WriteLine(e);
            }
        }
    }
}
