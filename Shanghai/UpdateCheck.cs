using MySql.Data.MySqlClient;

namespace Shanghai
{
    class UpdateCheck
    {
        private static readonly string TableName = "push_log";

        public UpdateCheck() { }

        public void Check(TaskServer server, string taskName)
        {
            using(var conn = DatabaseManager.OpenConnection())
            {
                var cmd = new MySqlCommand($"SELECT * from {TableName}", conn);
                using (var reader = cmd.ExecuteReader())
                {
                    while (reader.Read())
                    {
                        for(int i = 0; i < reader.FieldCount; i++)
                        {
                            Logger.Log(LogLevel.Info, "{0}: {1}",
                                reader.GetName(i), reader.GetValue(i).ToString());
                        }
                    }
                }
            }
        }
    }
}
