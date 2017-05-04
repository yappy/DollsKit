using System;
using System.Collections.Generic;
using System.Text;

namespace DollsLang
{
    public partial class Runtime
    {
        private StringBuilder outputBuffer;
        private Random random;

        private void InitializeRuntime()
        {
            outputBuffer.Clear();
            // initialize with current tick
            random = new Random();
        }

        private void LoadDefaultFunctionsInternal()
        {
            LoadFunction("print", LibPrint);
            LoadFunction("p", LibPrint);
            LoadFunction("for", LibFor);
            LoadFunction("foreach", LibForEach);
            LoadFunction("size", LibSize);
            LoadFunction("rand", LibRand);
        }

        private Value GetParam(Value[] args, int index)
        {
            if (index >= args.Length)
            {
                throw new RuntimeLangException(
                    $"Parameter #{index + 1} is required");
            }
            return args[index];
        }

        private Value LibPrint(Value[] args)
        {
            bool first = true;
            foreach (var value in args)
            {
                if (!first)
                {
                    outputBuffer.Append(' ');
                }
                first = false;
                outputBuffer.Append(value.ToString());
            }
            outputBuffer.Append('\n');
            // max size = OutputSize
            outputBuffer.Length = Math.Min(outputBuffer.Length, OutputSize);

            return NilValue.Nil;
        }

        private Value LibFor(Value[] args)
        {
            int start = GetParam(args, 0).ToInt();
            int end = GetParam(args, 1).ToInt();
            FunctionValue func = GetParam(args, 2).ToFunction();

            Value[] callArgs = new Value[1];
            for (int i = start; i <= end; i++)
            {
                callArgs[0] = new IntValue(i);
                CallFunction(func, callArgs);
            }

            return NilValue.Nil;
        }

        private Value LibForEach(Value[] args)
        {
            ArrayValue array = GetParam(args, 0).ToArray();
            List<Value> list = array.ValueList;
            FunctionValue func = GetParam(args, 1).ToFunction();

            Value[] callArgs = new Value[2];
            for (int i = 0; i < list.Count; i++)
            {
                callArgs[0] = list[i];
                callArgs[1] = new IntValue(i);
                CallFunction(func, callArgs);
            }

            return NilValue.Nil;
        }

        private Value LibSize(Value[] args)
        {
            ArrayValue array = GetParam(args, 0).ToArray();

            return new IntValue(array.ValueList.Count);
        }

        private Value LibRand(Value[] args)
        {
            switch (args.Length)
            {
                case 0:
                    return new FloatValue(random.NextDouble());
                case 1:
                    {
                        int maxValue = GetParam(args, 0).ToInt();
                        return new IntValue(random.Next(maxValue));
                    }
                default:
                    {
                        int minValue = GetParam(args, 0).ToInt();
                        int maxValue = GetParam(args, 1).ToInt();
                        return new IntValue(random.Next(minValue, maxValue));
                    }
            }
        }
    }
}
