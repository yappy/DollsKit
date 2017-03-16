using System;
using System.Diagnostics;
using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
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
            if (!settings.Enabled)
            {
                Logger.Log(LogLevel.Info,
                    "[{0}] DDNS update is disabled, skip", taskName);
                return;
            }

            const string Uri = "http://www.mydns.jp/login.html";
            const int TimeoutSec = 10;

            var client = new HttpClient();
            client.Timeout = TimeSpan.FromSeconds(TimeoutSec);
            byte[] byteArray = Encoding.ASCII.GetBytes(
                string.Format("{0}:{1}", settings.User, settings.Pass));
            client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
                "Basic", Convert.ToBase64String(byteArray));

            Task<HttpResponseMessage> task = client.GetAsync(Uri, server.CancelToken);
            task.Wait(server.CancelToken);
            if (task.Result.StatusCode == HttpStatusCode.OK)
            {
                Logger.Log(LogLevel.Info,
                    "[{0}] DDNS update succeeded", taskName);
            }
            else
            {
                Logger.Log(LogLevel.Warning,
                    "[{0}] DDNS update failed", taskName);
            }
        }

        [Obsolete]
        public void updateIpAddr(TaskServer server, string taskName)
        {
            const string Uri = "http://inet-ip.info";
            const string UserAgent = "curl";
            const int TimeoutSec = 10;

            var httpClient = new HttpClient();
            httpClient.DefaultRequestHeaders.TryAddWithoutValidation("User-Agent", UserAgent);
            httpClient.Timeout = TimeSpan.FromSeconds(TimeoutSec);

            Task<string> task = httpClient.GetStringAsync(Uri);
            task.Wait(server.CancelToken);

            string ipAddr = task.Result.Trim();
            Logger.Log(LogLevel.Info,
                "[{0}] IP addr: {1}", taskName, ipAddr);
        }
    }
}
