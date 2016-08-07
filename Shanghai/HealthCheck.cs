using System.IO;
using System.Text;

namespace Shanghai
{
    class HealthCheck
    {
        public HealthCheck()
        { }

        public void Proc(TaskServer server, string taskName)
        {
            var msg = new StringBuilder();
            msg.Append("[Health Check]\n");
            msg.AppendFormat("CPU Temp: {0:F3}\n", GetCpuTemp());
            TwitterManager.Update(msg.ToString());
        }

        private static double GetCpuTemp()
        {
            const string DevFilePath = "/sys/class/thermal/thermal_zone0/temp";
            string src;
            using (var reader = new StreamReader(DevFilePath))
            {
                src = reader.ReadLine();
            }
            return int.Parse(src) / 1000.0;
        }
    }
}
