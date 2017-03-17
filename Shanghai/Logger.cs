using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading;

namespace Shanghai
{
    // ログレベル
    // フィルタレベルより下ならば出力されない
    public enum LogLevel
    {
        Trace,
        Info,
        Warning,
        Error,
        Fatal,
    }

    // ロギングシステム
    // thread-safe
    public static class Logger
    {
        // 同期用オブジェクト
        private static object SyncObj = new object();
        // 出力先のリスト
        private static List<LogTarget> Targets = new List<LogTarget>();

        public static void AddConsole(LogLevel filterLevel)
        {
            lock (SyncObj)
            {
                Targets.Add(new ConsoleLogger(filterLevel));
            }
        }

        public static void AddFile(LogLevel filterLevel,
            string fileName = "log{0}.txt",
            long rotateSize = 10 * 1024 * 1024, int rotateNum = 2)
        {
            lock (SyncObj)
            {
                Targets.Add(new FileLogger(filterLevel,
                    fileName, rotateSize, rotateNum));
            }
        }

        public static void Log(LogLevel level, string value)
        {
            int thId = Thread.CurrentThread.ManagedThreadId;
            DateTime timestamp = DateTime.Now;

            lock (SyncObj)
            {
                foreach (var target in Targets)
                {
                    if (target.CheckLogLevel(level)) {
                        target.Write(value.ToString(), level, thId, timestamp);
                    }
                }
            }
        }

        public static void Log(LogLevel level, object value)
        {
            Log(level, value.ToString());
        }

        public static void Log(LogLevel level, string format, params object[] args){
            Log(level, string.Format(format, args));
        }

        public static void Flush()
        {
            lock (SyncObj)
            {
                foreach (var target in Targets)
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
            lock (SyncObj)
            {
                foreach (var target in Targets)
                {
                    try { target.Flush(); }
                    catch (Exception e) { Console.Error.WriteLine(e); }
                    try { target.Dispose(); }
                    catch (Exception e) { Console.Error.WriteLine(e); }
                }
                Targets.Clear();
            }
        }
    }

    // ログの出力先
    // 各メソッドは同期が取られた状態で呼び出される
    abstract class LogTarget
    {
        private LogLevel filterLevel;
        protected LogTarget(LogLevel filterLevel)
        {
            this.filterLevel = filterLevel;
        }

        // 与えられたレベルがフィルタより上ならtrue
        public bool CheckLogLevel(LogLevel level)
        {
            return (int)level >= (int)filterLevel;
        }

        // ログの1エントリを書き込む
        public abstract void Write(string msg,
            LogLevel level, int threadId, DateTime timestamp);
        // バッファリングしている場合はフラッシュする
        public abstract void Flush();
        // 後処理があるなら行う
        public abstract void Dispose();
    }

    class ConsoleLogger : LogTarget
    {
        public ConsoleLogger(LogLevel filterLevel) : base(filterLevel)
        {}

        public override void Write(string msg,
            LogLevel level, int threadId, DateTime timestamp)
        {
            Console.WriteLine(
                "{0} ({1}) [{2}]: {3}",
                timestamp, threadId, level, msg);
        }
        public override void Flush() { }
        public override void Dispose() { }
    }

    class FileLogger : LogTarget
    {
        private const int BufferMax = 64 * 1024;

        private string fileName;
        private long rotateSize;
        private int rotateNum;

        private StringBuilder buffer = new StringBuilder(BufferMax);

        public FileLogger(LogLevel filterLevel,
            string fileName, long rotateSize, int rotateNum)
            : base(filterLevel)
        {
            this.fileName = fileName;
            this.rotateSize = rotateSize;
            this.rotateNum = rotateNum;
        }

        public override void Write(string msg,
            LogLevel level, int threadId, DateTime timestamp)
        {
            // バッファ最大長を超えそうならフラッシュする
            string data = string.Format(
                "{0} ({1}) [{2}]: {3}",
                timestamp, threadId, level, msg);
            if (buffer.Length + data.Length > BufferMax)
            {
                Flush();
            }
            // 改行文字の分超えることがあるが気にしない
            // バッファ最大長を超えるログメッセージが来るとやはりはみ出すが気にしない
            buffer.AppendLine(data);
        }

        public override void Flush() {
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

        public override void Dispose() { }
    }
}
