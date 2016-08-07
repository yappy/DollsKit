using System;
using System.Diagnostics;
using System.Threading;
using System.Threading.Tasks;

namespace Shanghai
{
    enum ServerResult
    {
        None,
        Reboot,
        Shutdown,
        ErrorReboot,
        FatalShutdown,
    }

    class TaskParameter
    {
        public string Name { get; set; }
        public int StartSec { get; set; }
        public int PeriodSec { get; set; }
        // Infinity if ReleaseCount < 0
        public int ReleaseCount { get; set; }
        public Action<TaskServer, string> Proc { get; set; }

        public static TaskParameter Periodic(string name, int startSec, int periodSec, Action<TaskServer, string> proc)
        {
            var param = new TaskParameter();
            param.Name = name;
            param.StartSec = startSec;
            param.PeriodSec = periodSec;
            param.ReleaseCount = -1;
            param.Proc = proc;
            return param;
        }

        public static TaskParameter OneShot(string name, int startSec, Action<TaskServer, string> proc)
        {
            var param = new TaskParameter();
            param.Name = name;
            param.StartSec = startSec;
            param.PeriodSec = 0;
            param.ReleaseCount = 1;
            param.Proc = proc;
            return param;
        }
    }

    sealed class TaskServer
    {
        private readonly int MaxTasks;
        private readonly int HeartBeatSec;

        private ServerResult runResult = ServerResult.None;
        private CancellationTokenSource cancelTokenSource;
        private SemaphoreSlim taskSema;

        public CancellationToken CancelToken
        {
            get { return cancelTokenSource.Token; }
        }

        public TaskServer(int maxTasks, int heartBeatSec)
        {
            MaxTasks = maxTasks;
            HeartBeatSec = heartBeatSec;

            cancelTokenSource = new CancellationTokenSource();
            taskSema = new SemaphoreSlim(MaxTasks);
        }

        // thread safe
        public void RegisterTask(TaskParameter taskParam)
        {
            Log.Trace.TraceEvent(TraceEventType.Information, 0, "[{0}] Register periodic task ({1}sec, T={2})", taskParam.Name, taskParam.StartSec, taskParam.PeriodSec);

            // to be released from release thread
            Action taskProc = () =>
            {
                // get semaphore and do Proc()
                bool enter = taskSema.Wait(0);
                if (!enter)
                {
                    // Max count tasks are running, give up
                    Log.Trace.TraceEvent(TraceEventType.Warning, 0, "[{0}] Task count full, give up", taskParam.Name);
                    return;
                }
                try
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "[{0}] Start", taskParam.Name);
                    taskParam.Proc(this, taskParam.Name);
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "[{0}] Completed successfully", taskParam.Name);
                }
                catch (OperationCanceledException)
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "[{0}] Task cancelled", taskParam.Name);
                }
                catch (Exception e)
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "[{0}] Exception", taskParam.Name);
                    Log.Trace.TraceData(TraceEventType.Error, 0, e);
                }
                finally
                {
                    taskSema.Release();
                }
            };
            // release thread, which releases releaseProc
            Task.Run(() =>
            {
                try
                {
                    // delay, using CencelToken
                    Task.Delay(taskParam.StartSec * 1000).Wait(CancelToken);
                    int count = 0;
                    while (taskParam.ReleaseCount < 0 || count < taskParam.ReleaseCount)
                    {
                        // release task
                        Task.Run(taskProc);
                        // delay, using CencelToken
                        Task.Delay(taskParam.PeriodSec * 1000).Wait(CancelToken);
                        if (taskParam.ReleaseCount >= 0)
                        {
                            count++;
                        }
                    }
                }
                catch (OperationCanceledException)
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "[{0}-release-thread] Task cancelled", taskParam.Name);
                }
                catch (Exception e)
                {
                    Log.Trace.TraceEvent(TraceEventType.Information, 0, "[{0}-release-thread] Exception", taskParam.Name);
                    Log.Trace.TraceData(TraceEventType.Error, 0, e);
                }
            });
        }

        // shutdown from external
        // thread safe
        public void Shutdown(ServerResult request)
        {
            SetResult(request);
            cancelTokenSource.Cancel();
        }

        // use the most serious result
        // thread safe
        private void SetResult(ServerResult request)
        {
            if (request == ServerResult.None)
            {
                throw new ArgumentException("request must not be NONE");
            }
            lock (this)
            {
                int prev = (int)runResult;
                int next = Math.Max((int)request, prev);
                runResult = (ServerResult)next;
            }
        }

        public ServerResult Run(params TaskParameter[] tasks)
        {
            Array.ForEach(tasks, (task) =>
            {
                RegisterTask(task);
            });

            // Task Server main
            try
            {
                while (true)
                {
                    Log.Trace.TraceEvent(TraceEventType.Verbose, 0, "Heartbeat check...");
                    var start = DateTime.Now;
                    bool hasSema = taskSema.Wait(HeartBeatSec * 1000, CancelToken);
                    if (!hasSema)
                    {
                        // cannot take a semaphore for HeartBeatSec
                        // break a loop and goto finally
                        Log.Trace.TraceEvent(TraceEventType.Error, 0, "Heartbeat check NG");
                        SetResult(ServerResult.ErrorReboot);
                        break;
                    }
                    else
                    {
                        // release immediately and sleep for rest time
                        taskSema.Release();
                        Log.Trace.TraceEvent(TraceEventType.Verbose, 0, "Heartbeat check OK");
                        var end = DateTime.Now;
                        int rest = HeartBeatSec - (int)(end - start).TotalSeconds;
                        rest = Math.Max(rest, 0);
                        Thread.Sleep(rest * 1000);
                    }
                }
            }
            catch (OperationCanceledException)
            {
                // thrown by cancelToken
                // shutdown from an external task
                Log.Trace.TraceEvent(TraceEventType.Information, 0, "Interrupted by others");
            }
            catch (Exception e)
            {
                // unknown
                Log.Trace.TraceData(TraceEventType.Critical, 0, e);
                SetResult(ServerResult.ErrorReboot);
            }

            // enable cancel state
            cancelTokenSource.Cancel();
            // Get all the semaphore
            // forbid new tasks threads and wait for all of tasks exiting
            for (int i = 0; i < MaxTasks; i++)
            {
                bool enter = taskSema.Wait(HeartBeatSec * 1000);
                if (!enter)
                {
                    // timeout
                    // probably dead-locked thread is alive, so reboot is dangerous
                    Log.Trace.TraceEvent(TraceEventType.Critical, 0, "Timeout: Tasks still remain");
                    return ServerResult.FatalShutdown;
                }
            }
            // read the last state without locks
            Trace.Assert(runResult != ServerResult.None);
            return runResult;
        }
    }
}
