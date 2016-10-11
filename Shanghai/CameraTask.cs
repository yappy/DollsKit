using System;
using System.Diagnostics;
using System.IO;
using System.Drawing;
using System.Drawing.Imaging;

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

        // x:y:quality
        private static readonly int ThumbX = 160;
        private static readonly int ThumbY = 120;
        private static readonly int ThumbQuality = 100;
        private static readonly string ThumOption = string.Format(
            "{0}:{1}:{2}", ThumbX, ThumbY, ThumbQuality);

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
            string dirName = now.ToString("yyyyMMdd");
            string dirPath = Path.Combine(PicDir, dirName);
            string fileName = now.ToString("yyyyMMdd_HHmm") + ".jpg";
            string filePath = Path.Combine(PicDir, dirName, fileName);
            string thName = now.ToString("yyyyMMdd_HHmm") + "_th.jpg";
            string thPath = Path.Combine(PicDir, dirName, thName);

            Log.Trace.TraceEvent(TraceEventType.Information, 0,
                "Take a picture: {0}", filePath);

            Directory.CreateDirectory(dirPath);
            var p = Process.Start("raspistill",
                string.Format(@"-o ""{0}"" -thumb {1}", filePath, ThumOption));
            p.WaitForExit();

            // Create thumb
            var image = Image.FromFile(filePath);
            var thumb = image.GetThumbnailImage(ThumbX, ThumbY, null, IntPtr.Zero);
            thumb.Save(thPath, ImageFormat.Jpeg);
        }
    }
}
