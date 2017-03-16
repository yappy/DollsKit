using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace Shanghai
{
    public static class Logger
    {
        [Flags]
        public enum Option
        {
            None = 0x0,
            Console = 0x1,
            File = 0x2,
        }

        private static object syncObj = new object();
        private static List<LogTarget> targets = new List<LogTarget>();

        public static void Initialize(Option option,
            string fileName = "log{0}.txt",
            long rotateSize = 10 * 1024 * 1024, int rotateNum = 2)
        {
            Terminate();
            lock (syncObj)
            {
                if (option.HasFlag(Option.Console))
                {
                    targets.Add(new ConsoleLogger());
                }
                if (option.HasFlag(Option.File))
                {
                    targets.Add(new FileLogger(fileName, rotateSize, rotateNum));
                }
            }
        }

        public static void WriteLine(object value)
        {
            lock (syncObj)
            {
                foreach (var target in targets)
                {
                    target.WriteLine(value.ToString());
                }
            }
        }

        public static void Flush()
        {
            lock (syncObj)
            {
                foreach (var target in targets)
                {
                    try
                    {
                        target.Flush();
                    }
                    catch (Exception e)
                    {
                        Console.Error.WriteLine(e);
                    }
                }
            }
        }

        public static void Terminate()
        {
            lock (syncObj)
            {
                foreach (var target in targets)
                {
                    try { target.Flush(); }
                    catch (Exception e) { Console.Error.WriteLine(e); }
                    try { target.Dispose(); }
                    catch (Exception e) { Console.Error.WriteLine(e); }
                }
                targets.Clear();
            }
        }
    }

    interface LogTarget
    {
        void WriteLine(string msg);
        void Flush();
        void Dispose();
    }

    class ConsoleLogger : LogTarget
    {
        public void WriteLine(string msg)
        {
            Console.WriteLine(msg);
        }
        public void Flush() { }
        public void Dispose() { }
    }

    class FileLogger : LogTarget
    {
        private const int BufferMax = 64 * 1024;

        private string fileName;
        private long rotateSize;
        private int rotateNum;

        private StringBuilder buffer = new StringBuilder(BufferMax);

        public FileLogger(string fileName, long rotateSize, int rotateNum)
        {
            this.fileName = fileName;
            this.rotateSize = rotateSize;
            this.rotateNum = rotateNum;
        }

        public void WriteLine(string msg)
        {
            if (buffer.Length + msg.Length > BufferMax)
            {
                Flush();
            }
            buffer.AppendLine(msg);
        }

        public void Flush() {
            // 以降で例外が発生したとしてもバッファはクリアする
            string data = buffer.ToString();
            buffer.Clear();

            string mainFile = string.Format(fileName, 0);
            if (File.Exists(mainFile))
            {
                var info = new FileInfo(mainFile);
                // このまま書くとローテーションサイズを超える場合
                if (info.Length + data.Length > rotateSize)
                {
                    // 最後のを消す(存在しない場合はエラーにならない)
                    string lastFile = string.Format(fileName, rotateNum - 1);
                    File.Delete(lastFile);
                    // 1つずつ後ろにリネーム(上書きはできずエラーになるのでうまくやる)
                    for (int i = rotateNum - 2; i >= 0; i--)
                    {
                        string src = string.Format(fileName, i);
                        string dst = string.Format(fileName, i + 1);
                        if (File.Exists(src)) {
                            File.Move(src, dst);
                        }
                    }
                }
            }
            // 追記オープンして書き込む(ファイルがない場合は新規作成)
            File.AppendAllText(mainFile, data);
        }

        public void Dispose() { }
    }
}
