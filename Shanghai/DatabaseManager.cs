using MySql.Data.MySqlClient;

namespace Shanghai
{
    public class DatabaseSettings
    {
        public string Server { get; set; } = "";
        public string User { get; set; } = "";
        public string Pass { get; set; } = "";
        public string DbName { get; set; } = "";
    }

    static class DatabaseManager
    {
        private static DatabaseSettings settings;

        public static void Initialize()
        {
            settings = SettingManager.Settings.Database;
        }

        public static void Terminate()
        {
            settings = null;
        }

        // 呼び出し側で Dispose() すること
        public static MySqlConnection OpenConnection()
        {
            var connStr = new MySqlConnectionStringBuilder();
            connStr.CharacterSet = "utf8mb4";
            connStr.Server = settings.Server;
            connStr.UserID = settings.User;
            connStr.Password = settings.Pass;
            connStr.Database = settings.DbName;

            var conn = new MySqlConnection(connStr.ToString());
            conn.Open();

            return conn;
        }

    }
}
