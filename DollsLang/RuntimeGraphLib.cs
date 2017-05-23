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
        private static readonly Color BgColor = Color.AliceBlue;
        private static readonly Color DefaultColor = Color.Black;
        private static readonly float DefaultWidth = 1.0f;

        private Bitmap bitmap = new Bitmap(SizeW, SizeH);
        private Graphics g = null;
        private Color penColor = DefaultColor;
        private float penWidth = DefaultWidth;
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
            g.Clear(BgColor);
            penColor = DefaultColor;
            penWidth = DefaultWidth;
        }

        private void LoadGraphVariablesInternal()
        {
            
        }

        private void LoadGraphFunctionsInternal()
        {
            LoadFunction("pen", LibPen);
            LoadFunction("clear", LibClear);
            LoadFunction("line", LibLine);
            LoadFunction("draw", LibDraw);
        }

        private Color ArrayToColor(ArrayValue array)
        {
            int r = ReadArray(array, 0).ToInt();
            int g = ReadArray(array, 1).ToInt();
            int b = ReadArray(array, 2).ToInt();
            CheckIntRange("R", r, 0, 255);
            CheckIntRange("G", g, 0, 255);
            CheckIntRange("B", b, 0, 255);
            return Color.FromArgb(r, g, b);
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

        private Value LibPen(Value[] args)
        {
            ArrayValue colorArray = GetParam(args, 0).ToArray();
            double width = 1.0;
            if (args.Length >= 2)
            {
                width = GetParam(args, 1).ToFloat();
            }

            penColor = ArrayToColor(colorArray);
            penWidth = (float)width;

            return NilValue.Nil;
        }

        private Value LibClear(Value[] args)
        {
            graphicsEnabled = true;

            ArrayValue colorArray = GetParam(args, 0).ToArray();

            g.Clear(ArrayToColor(colorArray));

            return NilValue.Nil;
        }

        private Value LibLine(Value[] args)
        {
            graphicsEnabled = true;

            ArrayValue startPos = GetParam(args, 0).ToArray();

            using (var pen = new Pen(penColor, penWidth))
            {
                double x1 = ReadArray(startPos, 0).ToFloat();
                double y1 = ReadArray(startPos, 1).ToFloat();
                for (int i = 1; i < args.Length; i++)
                {
                    ArrayValue nextPos = GetParam(args, i).ToArray();
                    double x2 = ReadArray(nextPos, 0).ToFloat();
                    double y2 = ReadArray(nextPos, 1).ToFloat();
                    g.DrawLine(pen, Transform(x1, y1), Transform(x2, y2));
                    x1 = x2;
                    y1 = y2;
                }
            }

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

            using (var pen = new Pen(penColor, penWidth))
            {
                Point prev = call(init);
                for (int i = 1; i <= count; i++)
                {
                    double t = init + dt * i;
                    Point cur = call(t);
                    g.DrawLine(pen, prev, cur);
                    prev = cur;
                }
            }

            return NilValue.Nil;
        }
    }
}
