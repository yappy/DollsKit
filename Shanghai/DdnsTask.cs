using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Shanghai
{
    public class DdnsSettings
    {
        public static readonly string DefaultSetting = "FillHere";

        public bool Enabled { get; set; } = false;
        public string User { get; set; } = DefaultSetting;
        public string Pass { get; set; } = DefaultSetting;
    }

    class DdnsTask
    {
        private DdnsSettings settings;

        public DdnsTask()
        {
            settings = SettingManager.Settings.Ddns;
        }

        public void UpdateTask(TaskServer server, string taskName)
        {
            throw new NotImplementedException();
        }
    }
}
