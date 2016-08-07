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
            var msg = new StringBuilder("[Health Check]\n");

            msg.AppendFormat("CPU Temp: {0:F3}\n", GetCpuTemp());

            double free, total;
            GetDiskInfoG(out free, out total);
            msg.AppendFormat("Disk: {0:F3}G/{1:F3}G Free ({2}%)",
                free, total, (int)(free * 100.0 / total));

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

        private static void GetDiskInfoG(out double free, out double total)
        {
            const double Unit = 1024.0 * 1024.0 * 1024.0;
            var info = new DriveInfo("/");
            free = info.TotalFreeSpace / Unit;
            total = info.TotalSize / Unit;
        }
    }
}
