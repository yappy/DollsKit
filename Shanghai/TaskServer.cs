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
        // 時間以内に終わらないタスクがあったので再初期化
        ErrorReload,
        // プロセス終了
        Shutdown,
        // プロセス終了、デーモンにアップデート後再起動してもらう
        UpdateReboot,
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
        private static readonly int ShutdownTimeoutMs = 60 * 1000;

        private object SyncObj = new object();
        private ManualResetEventSlim SchedEvent = new ManualResetEventSlim();
        private bool Started;
        private ServerResult RunResult = ServerResult.None;
        private CancellationTokenSource CancelTokenSource = new CancellationTokenSource();
        private List<PeriodicTask> PeriodicTaskList = new List<PeriodicTask>();
        private List<OneShotTask> OneShotTaskList = new List<OneShotTask>();

        // タスクはこのトークンで RequestShutdown を検知できる
        public CancellationToken CancelToken
        {
            get { return CancelTokenSource.Token; }
        }

        public TaskServer()
        { }

        // 周期タスクの登録
        // サーバ開始前のみ可能 (thread safe)
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
        // thread safe: サーバ実行中も可能
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
                SchedEvent.Set();
            }
            Logger.Log(LogLevel.Info, "Register one-shot task: [{0}] ({1})", name, targetTime);
        }

        // 外部からのリブートやシャットダウン要求を受け付ける
        // thread safe
        public void RequestShutdown(ServerResult request)
        {
            if (request == ServerResult.None)
            {
                throw new ArgumentException("Request must not be NONE");
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
            // 周期タスクパラメータをロックしてからローカルにコピー
            PeriodicTask[] periodicTaskParams;
            Task[] periodicTaskExec;
            lock (SyncObj)
            {
                if (Started)
                {
                    throw new InvalidOperationException("Server is already started");
                }
                Started = true;
                periodicTaskParams = PeriodicTaskList.ToArray();
                periodicTaskExec = new Task[periodicTaskParams.Length];
            }
            // 実行中の単発タスク
            var oneShotTaskParams = new List<OneShotTask>();
            var oneShotTaskExec = new List<Task>();

            // 最後に周期タスクをスケジュールした分
            var lastMin = new DateTime(0);

            // Task Server main
            try
            {
                while (true)
                {
                    Debug.Assert(oneShotTaskParams.Count == oneShotTaskExec.Count);

                    /* 起床 */
                    Logger.Log(LogLevel.Trace, "sched!");

                    // 現在時間を取得
                    var now = DateTime.Now;
                    // 分以下を切り捨て
                    var currentMin = new DateTime(now.Year, now.Month, now.Day, now.Hour, now.Minute, 0);
                    // 次の分 (秒以下は0)
                    var nextMin = currentMin.AddMinutes(1);
                    // 何もなければ次の分+1秒に起きる
                    var timeoutTarget = nextMin.AddSeconds(1);

                    // (1) 終了したタスクを片付ける
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
                            i--;
                        }
                    }

                    // (2) タスクのリリース条件のチェックと実行開始
                    // 前回より分が進んでいるときのみ
                    if (currentMin > lastMin)
                    {
                        lastMin = currentMin;
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
                                    periodicTaskExec[i] =
                                        Task.Run(() => param.Proc(this, param.Name), CancelToken).
                                        ContinueWith((t) => SchedEvent.Set());
                                }
                                else
                                {
                                    // このタスクの前回の実行が終わっていない
                                    Logger.Log(LogLevel.Error, "[{0}] previous task is still running",
                                        periodicTaskParams[i].Name);
                                    // CancellationToken でタスクの中止を試みる
                                    // サーバや他のタスクも巻き込むのでサーバの再作成を要求する
                                    RequestShutdown(ServerResult.ErrorReload);
                                }
                            }
                        }
                    }
                    // 単発タスク
                    lock (SyncObj)
                    {
                        Func<OneShotTask, bool> cond = (elem) => now > elem.TargetTime;
                        // 開始条件を満たしたものを実行開始
                        foreach (var param in OneShotTaskList.Where(cond))
                        {
                            // サーバログを出してから実行開始し Task を oneShotTaskExec に入れる
                            // ! OperationCanceledException !
                            OnTaskStart(param.Name);
                            oneShotTaskParams.Add(param);
                            oneShotTaskExec.Add(
                                Task.Run(() => param.Proc(this, param.Name), CancelToken).
                                ContinueWith((t) => SchedEvent.Set()));
                        }
                        // 開始条件を満たしたものを削除
                        OneShotTaskList.RemoveAll((elem) => cond(elem));
                    }

                    // (3)
                    SchedEvent.Wait(timeoutTarget - now, CancelToken);
                    SchedEvent.Reset();
                } // while (true)
            }
            catch (OperationCanceledException)
            {
                Logger.Log(LogLevel.Info, "Server shutdown sequence");
                // シャットダウン要求が入った
                // 各タスクにもキャンセル要求が入っているので全て終了するまで待つ
                Task[] runningTasks = periodicTaskExec.Where((task) => task != null).
                    Concat(oneShotTaskExec).ToArray();
                Logger.Log(LogLevel.Info, $"Wait for all of tasks... ({runningTasks.Length})");
                if (runningTasks.Length != 0)
                {
                    try
                    {
                        bool complete = Task.WaitAll(runningTasks, ShutdownTimeoutMs);
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
                            if (!(inner is OperationCanceledException))
                            {
                                Logger.Log(LogLevel.Info, inner);
                            }
                            return true;
                        });
                    }
                }
                lock (SyncObj)
                {
                    Debug.Assert(RunResult != ServerResult.None);
                    Logger.Log(LogLevel.Info, "Server shutdown: {0}", RunResult);
                    return RunResult;
                }
            }
        }
    }
}
