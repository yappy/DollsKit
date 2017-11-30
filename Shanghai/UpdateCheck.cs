using MySql.Data.MySqlClient;
using System;
using System.Diagnostics;

namespace Shanghai
{
    class UpdateCheck
    {
        private static readonly string BuildCmd = "ruby";
        private static readonly string BuildArgs = "autobuild.rb {0}";
        private static readonly string BuildDir = "..";

        private static readonly string MsgCmdUpdateOn = "@update on";
        private static readonly string MsgCmdUpdateOff = "@update off";

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

        // build_log テーブルに追加
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

        private bool FilterPushLog(string gitRef, string headMsg)
        {
            bool result = false;
            // master のみデフォルト ON
            if (gitRef == "refs/heads/master")
            {
                result = true;
            }
            // コミットメッセージによるコントロール
            if (headMsg.IndexOf(MsgCmdUpdateOn) >= 0)
            {
                result = true;
            }
            if (headMsg.IndexOf(MsgCmdUpdateOff) >= 0)
            {
                result = false;
            }
            return result;
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

                Action<string, string> outFunc = (prefix, data) => {
                    if (data != null)
                    {
                        Logger.Log(LogLevel.Info, prefix + data);
                    }
                };
                p.OutputDataReceived += (sender, e) => outFunc("1> ", e.Data);
                p.ErrorDataReceived += (sender, e) => outFunc("2> ", e.Data);
                p.BeginOutputReadLine();
                p.BeginErrorReadLine();

                // 自身のビルドが終わらない場合にビルドプロセスを kill して
                // 正常稼働に戻るのはやばいので Cancel 不可で無限待ちとしてタスクサーバに任せる
                // 本当に固まった場合、管理プログラム全体の異常終了となる
                // タイムアウトなし版のみ、DataReceived イベントがもう来ないことを保証する
                p.WaitForExit();

                // 例外なしで完走
                // 終了コード 0 ならビルド成功
                return p.ExitCode == 0;
            }
        }

        private void UpdateReboot(TaskServer server, string taskName)
        {
            Logger.Log(LogLevel.Info, $"[{taskName}] Request UpdateReboot");

            TwitterManager.UpdateNoThrow(
                (e) => Logger.Log(LogLevel.Error, e),
                $"[{DateTime.Now}] Reboot...");

            server.RequestShutdown(ServerResult.UpdateReboot);
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
                    id = reader.GetInt32("id");
                    Logger.Log(LogLevel.Info, "[{0}] Find push to be built: id={1}",
                        taskName, id);
                    gitRef = reader.GetString("ref");
                    compareUrl = reader.GetString("compare");
                    headMsg = reader.GetString("head_msg");
                    // auto build 条件フィルタ
                    if (FilterPushLog(gitRef, headMsg))
                    {
                        find = true;
                        // 条件を満たす中で最初に見つかった1件のみを処理
                        break;
                    }
                }
            }
            // 処理対象があるときのみ
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

                // build_log に結果を追加
                DateTime finishTime = DateTime.Now;
                WriteBuildLog(conn, id, startTime, finishTime,
                    success, message);

                // tweet
                string time = (finishTime - startTime).ToString("c");
                string twmsg = $"{DateTime.Now}: \nResult: {message}\nTime: {time}";
                TwitterManager.UpdateNoThrow(
                    (e) => Logger.Log(LogLevel.Error, e),
                    twmsg);

                // ビルド成功時のみプロセスを再起動
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
