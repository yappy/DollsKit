using System;
using System.Diagnostics;
using System.IO;

namespace Shanghai
{
    public class CameraSettings
    {
        public bool Enabled { get; set; } = false;
    }

    class CameraTask
    {
        private static readonly string WebDir = "www";
        private static readonly string PicDir = Path.Combine(WebDir, "pics");
        private static readonly string TweetDir = Path.Combine(WebDir, "twque");

        private static CameraSettings settings
        {
            get {  return SettingManager.Settings.camera; }
        }

        public CameraTask()
        {}

        public void TakePictureTask(TaskServer server, string taskName)
        {
            if (!settings.Enabled)
            {
                return;
            }

            var now = DateTime.Now;
            string dirName = now.ToString("yyyyMMDD");
            string dirPath = Path.Combine(PicDir, dirName);
            string fileName = now.ToString("yyyyMMDD_HHMM.jpg");
            string filePath = Path.Combine(PicDir, dirName, fileName);

            Log.Trace.TraceEvent(TraceEventType.Information, 0,
                "Take a picture: {0}", filePath);

            Directory.CreateDirectory(dirPath);
            var p = Process.Start("raspistill",
                string.Format(@"-t 1 -o ""{0}""", filePath));
            p.WaitForExit();
        }
    }
}
