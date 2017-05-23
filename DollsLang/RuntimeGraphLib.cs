using System;
using System.Collections.Generic;
using System.Drawing;
using System.Text;

namespace DollsLang
{
    public partial class Runtime
    {
        private static readonly int SIZE_W = 256;
        private static readonly int SIZE_H = 256;
        private Bitmap bitmap = new Bitmap(SIZE_W, SIZE_H);
        private Graphics g = null;
        private bool graphicsEnabled = false;

        private Bitmap GetGraphicsResultInternal()
        {
            return graphicsEnabled ? bitmap : null;
        }

        private void InitializeGraphRuntimeInternal()
        {
            if (g != null)
            {
                g.Dispose();
            }
            g = Graphics.FromImage(bitmap);
            g.Clear(Color.White);
        }

        private void LoadGraphVariablesInternal()
        {
            
        }

        private void LoadGraphFunctionsInternal()
        {
            //LoadFunction("print", LibPrint);
        }
    }
}
