﻿using System;
using System.Collections.Generic;
using System.Linq;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace Shanghai
{
    class HealthCheck
    {
        public HealthCheck()
        { }

        public void Check(TaskServer server, string taskName)
        {
            var msg = new StringBuilder("[Health Check]\n");
            {
                msg.AppendFormat("CPU Temp: {0:F1}\n", GetCpuTemp());
            }
            {
                List<CpuUsage> cpuUsage = GetCpuUsage(server.CancelToken);
                // use the total CPU usage only
                foreach (var usage in cpuUsage.Take(1))
                {
                    msg.AppendFormat("{0}:{1:F1}% ", usage.Name, usage.UsagePercent);
                }
                msg.Append('\n');
            }
            {
                double memTotal = 0.0, memFree = 0.0, memAvail = 0.0;
                GetMemInfoM(ref memTotal, ref memFree, ref memAvail);
                msg.AppendFormat("Mem: {0:F1}/{1:F1}M Avail ({2:F1}%)\n",
                    memAvail, memTotal, memAvail * 100.0 / memTotal);
            }
            {
                GetDiskInfoG(out double diskFree, out double diskTotal);
                msg.AppendFormat("Disk: {0:F1}/{1:F1}G Free ({2:F1}%)\n",
                    diskFree, diskTotal, (int)(diskFree * 100.0 / diskTotal));
            }
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

        private struct CpuUsage
        {
            public string Name;
            public double UsagePercent;
        }
        private static List<CpuUsage> GetCpuUsage(CancellationToken cancelToken)
        {
            // 0.name
            // 1.user  2.nice  3.system  4.idle  5.iowait
            // 6.irq  7.softirq  8.steal  9.guest  10.guest_nice
            const string DevFilePath = "/proc/stat";
            const int DelayMs = 1000;
            var stat1 = new List<string[]>();
            var stat2 = new List<string[]>();

            Action<List<string[]>> getStat = (List<string[]> stat) =>
            {
                using (var reader = new StreamReader(DevFilePath))
                {
                    string line;
                    while ((line = reader.ReadLine()) != null)
                    {
                        string[] elems = line.Split(new char[] { ' ' },
                            StringSplitOptions.RemoveEmptyEntries);
                        if (elems.Length == 11 && elems[0].StartsWith("cpu"))
                        {
                            stat.Add(elems);
                        }
                    }
                }
            };
            getStat(stat1);
            Task.Delay(DelayMs).Wait(cancelToken);
            getStat(stat2);

            var result = new List<CpuUsage>();
            for (int i = 0; i < Math.Min(stat1.Count, stat2.Count); i++)
            {
                long total1 = stat1[i].Skip(1).Select(long.Parse).Sum();
                long total2 = stat2[i].Skip(1).Select(long.Parse).Sum();
                long idle1 = stat1[i].Skip(4).Take(1).Select(long.Parse).Sum();
                long idle2 = stat2[i].Skip(4).Take(1).Select(long.Parse).Sum();
                double usage = 1.0 - (double)(idle2 - idle1) / (total2 - total1);
                result.Add(new CpuUsage()
                {
                    Name = stat1[i][0],
                    UsagePercent = usage * 100.0,
                });
            }
            return result;
        }

        private static void GetMemInfoM(ref double total, ref double free, ref double available)
        {
            const string DevFilePath = "/proc/meminfo";
            using (var reader = new StreamReader(DevFilePath))
            {
                string line;
                while ((line = reader.ReadLine()) != null)
                {
                    string[] elems = line.Split(new char[] { ' ' },
                            StringSplitOptions.RemoveEmptyEntries);
                    if (elems.Length != 3 || elems[2] != "kB")
                    {
                        continue;
                    }
                    // kB -> mB
                    double value = long.Parse(elems[1]) / 1024.0;
                    switch (elems[0])
                    {
                        case "MemTotal:":
                            total = value;
                            break;
                        case "MemFree:":
                            free = value;
                            break;
                        case "MemAvailable:":
                            available = value;
                            break;
                    }
                }
            }
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
