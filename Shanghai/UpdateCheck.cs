using MySql.Data.MySqlClient;
using System;

namespace Shanghai
{
    class UpdateCheck
    {
        public UpdateCheck() { }

        // build_log 中の push_log id の最大値
        // これより大きな push_log id が未処理である
        // 無い場合は 0
        private int GetMaxPushIdInBuildLog(MySqlConnection conn)
        {
            int max = 0;
            var cmd = new MySqlCommand($"SELECT MAX(push_id) FROM build_log", conn);
            using (var reader = cmd.ExecuteReader())
            {
                while (reader.Read())
                {
                    int ordinal = reader.GetOrdinal("MAX(push_id)");
                    if (!reader.IsDBNull(ordinal))
                    {
                        max = reader.GetInt32(ordinal);
                    }
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

        private void BuildIfPushed(string taskName, MySqlConnection conn)
        {
            int maxIdInBuild = GetMaxPushIdInBuildLog(conn);
            Logger.Log(LogLevel.Info, "[{0}] Max push_id in build log: {1}",
                taskName, maxIdInBuild);

            var cmd = new MySqlCommand("SELECT MAX(id), ref, compare, head_msg " +
                "FROM push_log WHERE id > @max_id_in_build",
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
                    // NULL if not found
                    int ordinal = reader.GetOrdinal("MAX(id)");
                    if (reader.IsDBNull(ordinal))
                    {
                        continue;
                    }
                    // OK
                    id = reader.GetInt32("MAX(id)");
                    Logger.Log(LogLevel.Info, "[{0}] Find push to be built: id={1}",
                        taskName, id);
                    gitRef = reader.GetString("ref");
                    compareUrl = reader.GetString("compare");
                    headMsg = reader.GetString("head_msg");
                    find = true;
                }
            }
            // Build if found
            if (find)
            {
                DateTime startTime = DateTime.Now;
                bool success = false;
                try
                {
                    // TODO
                    TwitterManager.Update($"アップデートが見つかりました\n{gitRef}\n{compareUrl}");
                    success = true;
                }
                finally
                {
                    // Write build_log
                    DateTime finishTime = DateTime.Now;
                    WriteBuildLog(conn, id, startTime, finishTime,
                        success, success ? "OK" : "NG");
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
                BuildIfPushed(taskName, conn);
            }
        }
    }
}
