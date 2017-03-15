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
            long rotateSize = 5 * 1024 * 1024, int rotateNum = 2)
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
                    targets.Clear();
                }
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
            buffer.Append(msg);
        }

        public void Flush() {
            string mainFile = string.Format(fileName, 0);
            if (File.Exists(mainFile))
            {
                var info = new FileInfo(mainFile);
                if (info.Length + buffer.Length > rotateSize)
                {
                    // n-2 -> n-1, n-3 -> n-2, ..., 0 -> 1
                    for (int i = rotateNum - 2; i >= 0; i--)
                    {
                        string src = string.Format(fileName, i);
                        string dst = string.Format(fileName, i + 1);
                        File.Copy(src, dst, true);
                    }
                }
            }
            File.AppendAllText(mainFile, buffer.ToString());
            buffer.Clear();
        }

        public void Dispose() { }
    }
}
