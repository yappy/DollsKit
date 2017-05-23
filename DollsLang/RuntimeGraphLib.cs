using System;
using System.Collections.Generic;
using System.Drawing;
using System.Text;

namespace DollsLang
{
    public partial class Runtime
    {
        private static readonly int SizeW = 256;
        private static readonly int SizeH = 256;
        private static readonly Pen DefaultPen = Pens.Black;

        private Bitmap bitmap = new Bitmap(SizeW, SizeH);
        private Graphics g = null;
        private Pen pen = DefaultPen;
        private bool graphicsEnabled = false;

        private Bitmap GetGraphicsResultInternal()
        {
            return graphicsEnabled ? bitmap : null;
        }

        private void InitializeGraphRuntime()
        {
            graphicsEnabled = false;
            if (g != null)
            {
                g.Dispose();
            }
            g = Graphics.FromImage(bitmap);
            g.Clear(Color.White);
            pen = DefaultPen;
        }

        private void LoadGraphVariablesInternal()
        {
            
        }

        private void LoadGraphFunctionsInternal()
        {
            LoadFunction("line", LibLine);
            LoadFunction("draw", LibDraw);
        }

        private Point Transform(double x, double y)
        {
            // (SIZE / 2) + v * (SIZE / 2)
            // = (SIZE / 2) * (1 + v)
            // REVERSE y
            return new Point(
                (int)(SizeW / 2 * (1.0 + x)),
                (int)(SizeH / 2 * (1.0 - y)));
        }

        private Value LibLine(Value[] args)
        {
            graphicsEnabled = true;

            double x1 = GetParam(args, 0).ToFloat();
            double y1 = GetParam(args, 1).ToFloat();
            double x2 = GetParam(args, 2).ToFloat();
            double y2 = GetParam(args, 3).ToFloat();

            g.DrawLine(pen, Transform(x1, y1), Transform(x2, y2));

            return NilValue.Nil;
        }

        private Value LibDraw(Value[] args)
        {
            graphicsEnabled = true;

            double init = GetParam(args, 0).ToFloat();
            double end = GetParam(args, 1).ToFloat();
            int count = GetParam(args, 2).ToInt();
            FunctionValue func = GetParam(args, 3).ToFunction();
            if (count < 1)
            {
                throw new RuntimeLangException($"Count is less than 1: {count}");
            }

            double dt = (end - init) / count;

            Func<double, Point> call = (t) => {
                ArrayValue result = CallFunction(func, new FloatValue(t)).ToArray();
                double x = ReadArray(result, 0).ToFloat();
                double y = ReadArray(result, 1).ToFloat();
                return Transform(x, y);
            };

            Point prev = call(init);
            for (int i = 1; i <= count; i++)
            {
                double t = init + dt * i;
                Point cur = call(t);
                g.DrawLine(pen, prev, cur);
                prev = cur;
            }

            return NilValue.Nil;
        }
    }
}
