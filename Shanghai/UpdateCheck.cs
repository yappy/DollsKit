using MySql.Data.MySqlClient;
using System;
using System.Diagnostics;
using System.Text;

namespace Shanghai
{
    class UpdateCheck
    {
        private static readonly string BuildCmd = "ruby";
        private static readonly string BuildArgs = "autobuild.rb {0}";
        private static readonly string BuildDir = "..";

        public UpdateCheck() { }

        // build_log 中の push_log id の最大値
        // これより大きな push_log id が未処理である
        // 無い場合は 0
        private int GetMaxPushIdInBuildLog(MySqlConnection conn)
        {
            int max = 0;
            var cmd = new MySqlCommand(
                "SELECT push_id FROM build_log " +
                "WHERE push_id = (SELECT MAX(push_id) FROM build_log)",
                conn);
            using (var reader = cmd.ExecuteReader())
            {
                while (reader.Read())
                {
                    max = reader.GetInt32("push_id");
                }
            }
            return max;
        }

        private void WriteBuildLog(MySqlConnection conn,
            int pushId, DateTime startedAt, DateTime finishedAt,
            bool succeeded, string message)
        {
            var cmd = new MySqlCommand("INSERT INTO build_log " + 
                "(push_id, started_at, finished_at, succeeded, message) " +
                "VALUES (@push_id, @started_at, @finished_at, @succeeded, @message)",
                conn);
            cmd.Prepare();
            cmd.Parameters.AddWithValue("@push_id", pushId);
            cmd.Parameters.AddWithValue("@started_at", startedAt);
            cmd.Parameters.AddWithValue("@finished_at", finishedAt);
            cmd.Parameters.AddWithValue("@succeeded", succeeded);
            cmd.Parameters.AddWithValue("@message", message);
            cmd.ExecuteNonQuery();
        }

        // true: 正常終了
        // false: ビルドプロセスは完走したが失敗
        // Exception: プロセス起動失敗等のシステムエラー
        private bool Build(string taskName, string gitRef)
        {
            // バックスラッシュとダブルクォートを消してダブルクォートで括る
            // 正常ケースではそんな文字は出てこないのでセキュリティ問題だけ回避しておく
            gitRef = gitRef.Replace('\\', ' ').Replace('"', ' ');
            gitRef = '"' + gitRef + '"';

            var startInfo = new ProcessStartInfo();
            startInfo.FileName = BuildCmd;
            startInfo.Arguments = string.Format(BuildArgs, gitRef);
            startInfo.WorkingDirectory = BuildDir;
            startInfo.UseShellExecute = false;
            startInfo.RedirectStandardInput = true;
            startInfo.RedirectStandardOutput = true;
            startInfo.RedirectStandardError = true;
            Logger.Log(LogLevel.Info,
                "[{0}] FileName = {1}, Args = {2}, WorkDir = {3}",
                taskName,
                startInfo.FileName, startInfo.Arguments, startInfo.WorkingDirectory);

            using (var p = Process.Start(startInfo))
            {
                // stdin は即 EOF
                p.StandardInput.Close();

                Action<string> outFunc = (line) => {
                    Logger.Log(LogLevel.Info, line);
                };
                p.OutputDataReceived += (sender, e) => outFunc("1> " + e.Data);
                p.ErrorDataReceived += (sender, e) => outFunc("2> " + e.Data);
                p.BeginOutputReadLine();
                p.BeginErrorReadLine();

                // 自身のビルドが終わらない場合にビルドプロセスを kill して
                // 正常稼働に戻るのはやばいので Cancel 不可で無限待ちとしてタスクサーバに任せる
                // 本当に固まった場合、管理プログラム全体の異常終了となる
                p.WaitForExit();

                // 例外なしで完走
                // 終了コード 0 ならビルド成功
                return p.ExitCode == 0;
            }
        }

        private void UpdateReboot(TaskServer server, string taskName)
        {
            Logger.Log(LogLevel.Info, $"[{taskName}] Request UpdateShutdown");

            TwitterManager.UpdateNoThrow(
                (e) => Logger.Log(LogLevel.Error, e),
                $"[{DateTime.Now}] Shutdown...");

            server.RequestShutdown(ServerResult.UpdateShutdown);
        }

        private void BuildIfPushed(TaskServer server, string taskName, MySqlConnection conn)
        {
            int maxIdInBuild = GetMaxPushIdInBuildLog(conn);
            Logger.Log(LogLevel.Info, "[{0}] Max push_id in build log: {1}",
                taskName, maxIdInBuild);

            // 最終ビルドより後の push のみを新しいものから先に列挙
            var cmd = new MySqlCommand(
                "SELECT id, ref, compare, head_msg " +
                "FROM push_log " +
                "WHERE id > @max_id_in_build " +
                "ORDER BY id DESC",
                conn);
            cmd.Prepare();
            cmd.Parameters.AddWithValue("@max_id_in_build", maxIdInBuild);

            bool find = false;
            int id = 0;
            string gitRef = null;
            string compareUrl = null;
            string headMsg = null;
            using (var reader = cmd.ExecuteReader())
            {
                while (reader.Read())
                {
                    // OK
                    id = reader.GetInt32("id");
                    Logger.Log(LogLevel.Info, "[{0}] Find push to be built: id={1}",
                        taskName, id);
                    gitRef = reader.GetString("ref");
                    compareUrl = reader.GetString("compare");
                    headMsg = reader.GetString("head_msg");
                    // TODO: headMsg で制御
                    find = true;
                    // 条件を満たす中で最初に見つかった1件のみを処理
                    break;
                }
            }
            // build if found
            if (find)
            {
                DateTime startTime = DateTime.Now;
                bool success = false;
                string message = "";
                try
                {
                    TwitterManager.UpdateNoThrow(
                        (e) => Logger.Log(LogLevel.Error, e),
                        $"アップデートが見つかりました\n{gitRef}\n{compareUrl}");

                    success = Build(taskName, gitRef);
                    message = success ? "Build OK" : "Build NG";
                }
                catch (Exception e)
                {
                    success = false;
                    message = "System error\n";
                    Logger.Log(LogLevel.Error, e);
                }

                // write build_log
                DateTime finishTime = DateTime.Now;
                WriteBuildLog(conn, id, startTime, finishTime,
                    success, message);

                // tweet
                string time = (finishTime - startTime).ToString("c");
                string twmsg = $"{DateTime.Now}: \n結果: {message}\n時間: {time}";
                TwitterManager.UpdateNoThrow(
                    (e) => Logger.Log(LogLevel.Error, e),
                    twmsg);

                // reboot if succeeded
                if (success)
                {
                    UpdateReboot(server, taskName);
                }
            }
            else
            {
                Logger.Log(LogLevel.Info, "[{0}] New push not found", taskName);
            }
        }

        public void Check(TaskServer server, string taskName)
        {
            using(var conn = DatabaseManager.OpenConnection())
            {
                BuildIfPushed(server, taskName, conn);
            }
        }
    }
}
