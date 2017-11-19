using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

namespace Shanghai
{
    // タスクサーバの終了結果
    // 同時に起きた場合は下の方が優先される
    public enum ServerResult
    {
        None,
        // プロセスそのままで再初期化
        Reload,
        // プロセス終了
        Shutdown,
        // プロセス終了、デーモンにアップデート後再起動してもらう
        UpdateReboot,
        // エラーでプロセス終了、デーモンに再起動してもらう
        ErrorReboot,
        // 致命的エラーによりプロセス終了
        FatalShutdown,
    }

    struct PeriodicTask
    {
        public string Name;
        // (hour, min) => bool
        public Func<int, int, bool> Condition;
        // (server, taskName)
        public Action<TaskServer, string> Proc;
    }

    struct OneShotTask
    {
        public string Name;
        public DateTime TargetTime;
        // (server, taskName)
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
        private List<OneShotTask> OneShotTaskList;

        // タスクはこのトークンで RequestShutdown を検知できる
        public CancellationToken CancelToken
        {
            get { return CancelTokenSource.Token; }
        }

        public bool Started { get; private set; }

        public TaskServer()
        {
            PeriodicTaskList = new List<PeriodicTask>();
            OneShotTaskList = new List<OneShotTask>();
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
            Logger.Log(LogLevel.Info, "Register periodic task: [{0}]", name);
        }

        // 単発タスクの登録
        // サーバ実行中も可能
        public void RegisterOneShotTask(string name,
            TimeSpan delay, Action<TaskServer, string> proc)
        {
            DateTime targetTime = DateTime.Now + delay;
            lock (SyncObj)
            {
                OneShotTaskList.Add(new OneShotTask
                {
                    Name = name,
                    TargetTime = targetTime,
                    Proc = proc,
                });
            }
            Logger.Log(LogLevel.Info, "Register one-shot task: [{0}] ({1})", name, targetTime);
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

        // タスク開始時のサーバログ
        private void OnTaskStart(string taskName)
        {
            Logger.Log(LogLevel.Info, "[{0}] Start", taskName);
        }

        // タスク終了時のサーバログ
        private void OnTaskEnd(string taskName, Task task)
        {
            switch (task.Status)
            {
                case TaskStatus.RanToCompletion:
                    Logger.Log(LogLevel.Info, "[{0}] End (Success)", taskName);
                    break;
                case TaskStatus.Canceled:
                    Logger.Log(LogLevel.Info, "[{0}] End (Canceled)", taskName);
                    break;
                case TaskStatus.Faulted:
                    Logger.Log(LogLevel.Info, "[{0}] End (Failed)", taskName);
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
            Task[] periodicTaskExec;
            lock (SyncObj) {
                if (Started) {
                    throw new InvalidOperationException("Server is already started");
                }
                Started = true;
                periodicTaskParams = PeriodicTaskList.ToArray();
                periodicTaskExec = new Task[periodicTaskParams.Length];
            }
            // 実行中の単発タスク
            var oneShotTaskParams = new List<OneShotTask>();
            var oneShotTaskExec = new List<Task>();

            // Task Server main
            try
            {
                while (true)
                {
                    Debug.Assert(oneShotTaskParams.Count == oneShotTaskExec.Count);

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

                        // WaitAny のため全実行中タスクからなる配列を作成
                        // periodicTaskArray から null を除いたもの + oneShotTaskExec
                        Task[] runningTasks = periodicTaskExec.Where((task) => task != null).Concat(
                            oneShotTaskExec).ToArray();
                        if (runningTasks.Length != 0)
                        {
                            // どれかのタスクが終了するか timeout 時間が経過するまで待機
                            // ! OperationCanceledException !
                            Task.WaitAny(runningTasks, timeout, CancelToken);
                            // 全部について終わっているか調べる
                            // 終わっているものについてはログを出して削除
                            for (int i = 0; i < periodicTaskExec.Length; i++)
                            {
                                if (periodicTaskExec[i] != null && periodicTaskExec[i].IsCompleted)
                                {
                                    OnTaskEnd(periodicTaskParams[i].Name, periodicTaskExec[i]);
                                    periodicTaskExec[i] = null;
                                }
                            }
                            for (int i = 0; i < oneShotTaskExec.Count; i++)
                            {
                                if (oneShotTaskExec[i].IsCompleted)
                                {
                                    OnTaskEnd(oneShotTaskParams[i].Name, oneShotTaskExec[i]);
                                    oneShotTaskParams.RemoveAt(i);
                                    oneShotTaskExec.RemoveAt(i);
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
                    // 周期タスク
                    for (int i = 0; i < periodicTaskParams.Length; i++)
                    {
                        // 時/分 で判定
                        if (periodicTaskParams[i].Condition(now.Hour, now.Minute))
                        {
                            if (periodicTaskExec[i] == null)
                            {
                                // コピーしてからスレッドへキャプチャ
                                var param = periodicTaskParams[i];
                                // サーバログを出してから実行開始し Task を periodicTaskExec[i] に入れる
                                // ! OperationCanceledException !
                                OnTaskStart(param.Name);
                                periodicTaskExec[i] = Task.Run(
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
                    // 単発タスク
                    lock (SyncObj)
                    {
                        Func<OneShotTask, bool> cond = (elem) => now > elem.TargetTime;
                        Predicate<OneShotTask> delCond = (elem) => cond(elem);
                        // 開始条件を満たしたものを実行開始
                        foreach (var param in OneShotTaskList.Where(cond))
                        {
                            // サーバログを出してから実行開始し Task を oneShotTaskExec に入れる
                            // ! OperationCanceledException !
                            OnTaskStart(param.Name);
                            oneShotTaskParams.Add(param);
                            oneShotTaskExec.Add(Task.Run(
                                () => param.Proc(this, param.Name),
                                CancelToken));
                        }
                        // 開始条件を満たしたものを削除
                        OneShotTaskList.RemoveAll(delCond);
                    }
                } // while (true)
            }
            catch (OperationCanceledException)
            {
                Logger.Log(LogLevel.Info, "Server shutdown sequence");
                // シャットダウン要求が入った
                // 各タスクにもキャンセル要求が入っているので全て終了するまで待つ
                Logger.Log(LogLevel.Info, "Wait for all of tasks...");
                Task[] runningTasks = periodicTaskExec.Where((task) => task != null).ToArray();
                if (runningTasks.Length != 0) {
                    try {
                        bool complete = Task.WaitAll(runningTasks, ShutdownTimeout);
                        if (complete)
                        {
                            // case 1: 全て正常終了した
                            Logger.Log(LogLevel.Info, "OK: succeeded all");
                        }
                        else
                        {
                            // case 3: WaitAll がタイムアウトした
                            // 実行中のタスクが残っている プロセス終了しかない
                            Logger.Log(LogLevel.Fatal, "Timeout: waiting for all of tasks");
                            return ServerResult.FatalShutdown;
                        }
                    }
                    catch (AggregateException e)
                    {
                        // case 2: 全て完了したが1つ以上がキャンセルまたは例外で終わった
                        Logger.Log(LogLevel.Info, "OK: one or more tasks are canceled or failed");
                        e.Flatten().Handle((inner) => {
                            if (!(inner is OperationCanceledException)) {
                                Logger.Log(LogLevel.Info, inner);
                            }
                            return true;
                        });
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
