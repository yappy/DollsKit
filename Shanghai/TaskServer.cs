using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

namespace Shanghai
{
    public enum ServerResult
    {
        None,
        Reboot,
        Shutdown,
        ErrorReboot,
        FatalShutdown,
    }

    struct PeriodicTask
    {
        public string Name;

        // (hour, min) => bool
        public Func<int, int, bool> Condition;
        // (serverm, taskName)
        public Action<TaskServer, string> Proc;
    }

    public class TaskServer
    {
        // シャットダウン要求が入ってから全タスクの終了を待つ時間
        // タイムアウトした場合実行中のタスクが残っているため fatal
        // プロセス終了するしかない
        private static readonly int ShutdownTimeout = 60 * 1000;

        private object SyncObj = new object();
        private ServerResult RunResult = ServerResult.None;
        private CancellationTokenSource CancelTokenSource;
        private List<PeriodicTask> PeriodicTaskList;

        // タスクはこのトークンで RequestShutdown を検知できる
        public CancellationToken CancelToken
        {
            get { return CancelTokenSource.Token; }
        }

        public bool Started { get; private set; }

        public TaskServer()
        {
            PeriodicTaskList = new List<PeriodicTask>();
            CancelTokenSource = new CancellationTokenSource();
        }

        // 周期タスクの登録
        // サーバ開始前のみ可能
        public void RegisterPeriodicTask(string name,
            Func<int, int, bool> condition, Action<TaskServer, string> proc)
        {
            lock (SyncObj)
            {
                if (Started)
                {
                    throw new InvalidOperationException("Server is started");
                }
                PeriodicTaskList.Add(new PeriodicTask {
                    Name = name,
                    Condition = condition,
                    Proc = proc,
                });
            }
        }

        // 外部からのリブートやシャットダウン要求を受け付ける
        // thread safe
        public void RequestShutdown(ServerResult request)
        {
            if (request == ServerResult.None)
            {
                throw new ArgumentException("request must not be NONE");
            }
            SetResult(request);
            CancelTokenSource.Cancel();
        }

        // 最も深刻な結果に上書きする
        // thread safe
        private void SetResult(ServerResult request)
        {
            lock (SyncObj)
            {
                int prev = (int)RunResult;
                int next = Math.Max((int)request, prev);
                RunResult = (ServerResult)next;
            }
        }

        // 周期タスク開始時のサーバログ
        private void OnPeriodicTaskStart(PeriodicTask param)
        {
            Logger.Log(LogLevel.Info, "[{0}] Start", param.Name);
        }

        // 周期タスク終了時のサーバログ
        private void OnPeriodicTaskEnd(PeriodicTask param, Task task)
        {
            switch (task.Status)
            {
                case TaskStatus.RanToCompletion:
                    Logger.Log(LogLevel.Info, "[{0}] End (Success)", param.Name);
                    break;
                case TaskStatus.Canceled:
                    Logger.Log(LogLevel.Info, "[{0}] End (Canceled)", param.Name);
                    break;
                case TaskStatus.Faulted:
                    Logger.Log(LogLevel.Info, "[{0}] End (Failed)", param.Name);
                    task.Exception.Flatten().Handle((e) =>
                    {
                        Logger.Log(LogLevel.Error, e);
                        return true;
                    });
                    break;
                default:
                    Debug.Assert(false);
                    break;
            }
        }

        public ServerResult Run()
        {
            // 周期タスクパラメータをロックしてからコピーして以降固定する
            PeriodicTask[] periodicTaskParams;
            Task[] periodicTaskArray;
            lock (SyncObj) {
                if (Started) {
                    throw new InvalidOperationException("Server is started");
                }
                Started = true;
                periodicTaskParams = PeriodicTaskList.ToArray();
                periodicTaskArray = new Task[periodicTaskParams.Length];
            }

            // Task Server main
            try
            {
                while (true)
                {
                    // 現在時間を取得
                    var now = DateTime.Now;
                    // 次の分 (秒以下は0)
                    var target = new DateTime(now.Year, now.Month, now.Day, now.Hour, now.Minute, 0).AddMinutes(1);
                    // 次の分+1秒を狙って sleep する
                    var timeoutTarget = target.AddSeconds(1);

                    // (1) target 時間まで待機
                    do
                    {
                        // timeout = timeoutTarget までの時間
                        now = DateTime.Now;
                        int timeout = Math.Max((int)(timeoutTarget - now).TotalMilliseconds, 0);
                        // WaitAny のため periodicTaskArray から null を除いた配列を作成
                        Task[] runningTasks = periodicTaskArray.Where((task) => task != null).ToArray();
                        if (runningTasks.Length != 0)
                        {
                            // どれかのタスクが終了するか timeout 時間が経過するまで待機
                            // ! OperationCanceledException !
                            Task.WaitAny(runningTasks, timeout, CancelToken);
                            // 全部について終わっているか調べる
                            // 終わっているものについてはログを出して削除
                            for (int i = 0; i < periodicTaskArray.Length; i++)
                            {
                                if (periodicTaskArray[i] != null && periodicTaskArray[i].IsCompleted)
                                {
                                    OnPeriodicTaskEnd(periodicTaskParams[i], periodicTaskArray[i]);
                                    periodicTaskArray[i] = null;
                                }
                            }
                        }
                        else
                        {
                            // 0個の配列では WaitAny が使えないので単に timeout だけ sleep
                            // thread-pool はタスクの実行で全部使われて遅延する可能性があるので
                            // サーバの一定時間待ちには Task.Delay は使わない
                            do
                            {
                                // 1秒毎に外部からのキャンセルをポーリング
                                Thread.Sleep(1000);
                                // ! OperationCanceledException !
                                CancelToken.ThrowIfCancellationRequested();
                                now = DateTime.Now;
                            } while (now < target);
                        }
                        // target 時間になるまで繰り返す
                        // (特に WaitAny でいずれかのタスクが完了して抜けた場合)
                        now = DateTime.Now;
                    } while (now < target);

                    // (2) タスクのリリース条件のチェックと実行開始
                    for (int i = 0; i < periodicTaskParams.Length; i++)
                    {
                        // 時/分 で判定
                        if (periodicTaskParams[i].Condition(now.Hour, now.Minute))
                        {
                            if (periodicTaskArray[i] == null)
                            {
                                // コピーしてからスレッドへキャプチャ
                                var param = periodicTaskParams[i];
                                // サーバログを出してから実行開始し Task を tasks[i] に入れる
                                // ! OperationCanceledException !
                                OnPeriodicTaskStart(param);
                                periodicTaskArray[i] = Task.Run(
                                    () => param.Proc(this, param.Name),
                                    CancelToken);
                            }
                            else
                            {
                                // このタスクの前回の実行が終わっていない
                                Logger.Log(LogLevel.Error, "[{0}] previous task is still running",
                                    periodicTaskParams[i].Name);
                                // CancellationToken でタスクの中止を試みる
                                // サーバや他のタスクも巻き込むのでサーバの再作成を要求する
                                RequestShutdown(ServerResult.ErrorReboot);
                            }
                        }
                    }
                } // while (true)
            }
            catch (OperationCanceledException)
            {
                Logger.Log(LogLevel.Info, "Server interrupted");
                // シャットダウン要求が入った
                // 各タスクにもキャンセル要求が入っているので全て終了するまで待つ
                Task[] runningTasks = periodicTaskArray.Where((task) => task != null).ToArray();
                if (runningTasks.Length != 0) {
                    try {
                        bool complete = Task.WaitAll(runningTasks, ShutdownTimeout);
                        if (complete)
                        {
                            // case 1: 全て正常終了した
                            Logger.Log(LogLevel.Info, "");
                        }
                        else
                        {
                            // case 3: WaitAll がタイムアウトした
                            // 実行中のタスクが残っている プロセス終了しかない
                            Logger.Log(LogLevel.Fatal, "Timeout: waiting for all tasks");
                            return ServerResult.FatalShutdown;
                        }
                    }
                    catch (AggregateException e)
                    {
                        // case 2: 全て完了したが1つ以上がキャンセルまたは例外で終わった
                        Logger.Log(LogLevel.Info, e.Flatten());
                    }
                }
                lock (SyncObj) {
                    Debug.Assert(RunResult != ServerResult.None);
                    Logger.Log(LogLevel.Info, "Server shutdown: {0}", RunResult);
                    return RunResult;
                }
            }
        }
    }
}
