using System;
using System.Collections.Generic;

namespace DollsLang
{
    public enum ValueType
    {
        Nil, Bool, Int, Float, String,
        Array,
        NativeFunction, UserFunction,
    }

    public abstract class Value
    {
        public static readonly int ShortStringMax = 16;
        public ValueType Type { get; private set; }

        protected Value(ValueType type)
        {
            Type = type;
        }

        protected string ToShortString()
        {
            string org = ToString();
            return org.Length < ShortStringMax ?
                org :
                org.Substring(0, ShortStringMax) + "...";
        }

        public abstract bool ToBool();
        public abstract int ToInt();
        public abstract double ToFloat();
        public abstract ArrayValue ToArray();
        public abstract FunctionValue ToFunction();
    }

    public class NilValue : Value
    {
        public static NilValue Nil { get; } = new NilValue();

        private NilValue()
            : base(ValueType.Nil)
        { }

        public override bool ToBool()
        {
            return false;
        }

        public override int ToInt()
        {
            throw new RuntimeLangException("Cannot convert to int: " + ToShortString());
        }

        public override double ToFloat()
        {
            throw new RuntimeLangException("Cannot convert to float: " + ToShortString());
        }

        public override string ToString()
        {
            return "nil";
        }

        public override ArrayValue ToArray()
        {
            throw new RuntimeLangException("Cannot convert to array: " + ToShortString());
        }

        public override FunctionValue ToFunction()
        {
            throw new RuntimeLangException("Cannot convert to function: " + ToShortString());
        }
    }

    public class BoolValue : Value
    {
        public bool RawValue { get; private set; }

        public static BoolValue False { get; } = new BoolValue(false);
        public static BoolValue True { get; } = new BoolValue(true);

        public static Value Of(bool b)
        {
            return b ? True : False;
        }

        private BoolValue(bool b)
            : base(ValueType.Bool)
        {
            RawValue = b;
        }

        public override bool ToBool()
        {
            return RawValue;
        }

        public override int ToInt()
        {
            throw new RuntimeLangException("Cannot convert to int: " + ToShortString());
        }

        public override double ToFloat()
        {
            throw new RuntimeLangException("Cannot convert to float " + ToShortString());
        }

        public override string ToString()
        {
            return RawValue ? "true" : "false";
        }

        public override ArrayValue ToArray()
        {
            throw new RuntimeLangException("Cannot convert to array " + ToShortString());
        }

        public override FunctionValue ToFunction()
        {
            throw new RuntimeLangException("Cannot convert to function " + ToShortString());
        }
    }

    public class IntValue : Value
    {
        public int RawValue { get; private set; }

        public IntValue(int value)
            : base(ValueType.Int)
        {
            RawValue = value;
        }

        public override bool ToBool()
        {
            return true;
        }

        public override int ToInt()
        {
            return RawValue;
        }

        public override double ToFloat()
        {
            return RawValue;
        }

        public override string ToString()
        {
            return RawValue.ToString();
        }

        public override ArrayValue ToArray()
        {
            throw new RuntimeLangException("Cannot convert to array: " + ToShortString());
        }

        public override FunctionValue ToFunction()
        {
            throw new RuntimeLangException("Cannot convert to function: " + ToShortString());
        }
    }

    public class FloatValue : Value
    {
        public double RawValue { get; private set; }

        public FloatValue(double value)
            : base(ValueType.Float)
        {
            RawValue = value;
        }

        public override bool ToBool()
        {
            return true;
        }

        public override int ToInt()
        {
            throw new RuntimeLangException("Cannot convert to int: " + ToShortString());
        }

        public override double ToFloat()
        {
            return RawValue;
        }

        public override string ToString()
        {
            return RawValue.ToString();
        }

        public override ArrayValue ToArray()
        {
            throw new RuntimeLangException("Cannot convert to array " + ToShortString());
        }

        public override FunctionValue ToFunction()
        {
            throw new RuntimeLangException("Cannot convert to function: " + ToShortString());
        }
    }

    public class StringValue : Value
    {
        public string RawValue { get; private set; }

        public StringValue(string value)
            : base(ValueType.String)
        {
            RawValue = value;
        }

        public override bool ToBool()
        {
            return true;
        }

        public override int ToInt()
        {
            int result;
            if (int.TryParse(RawValue, out result))
            {
                return result;
            }
            else
            {
                throw new RuntimeLangException("Cannot convert to int: " + ToShortString());
            }
        }

        public override double ToFloat()
        {
            double result;
            if (double.TryParse(RawValue, out result))
            {
                return result;
            }
            else
            {
                throw new RuntimeLangException("Cannot convert to float: " + ToShortString());
            }
        }

        public override string ToString()
        {
            return RawValue;
        }

        public override ArrayValue ToArray()
        {
            throw new RuntimeLangException("Cannot convert to array: " + ToShortString());
        }

        public override FunctionValue ToFunction()
        {
            throw new RuntimeLangException("Cannot convert to function: " + ToShortString());
        }
    }

    public class ArrayValue : Value
    {
        public List<Value> ValueList { get; private set; }

        public ArrayValue(List<Value> valueList)
            : base(ValueType.Array)
        {
            ValueList = valueList;
        }

        public override bool ToBool()
        {
            return true;
        }

        public override int ToInt()
        {
            throw new RuntimeLangException("Cannot convert to int: " + ToShortString());
        }

        public override double ToFloat()
        {
            throw new RuntimeLangException("Cannot convert to float: " + ToShortString());
        }

        public override string ToString()
        {
            return "[" + string.Join(",", ValueList) +"]";
        }

        public override ArrayValue ToArray()
        {
            return this;
        }

        public override FunctionValue ToFunction()
        {
            throw new RuntimeLangException("Cannot convert to function: " + ToShortString());
        }
    }

    public abstract class FunctionValue : Value
    {
        protected FunctionValue(ValueType type)
            : base(type)
        { }

        public override bool ToBool()
        {
            return true;
        }

        public override int ToInt()
        {
            throw new RuntimeLangException("Cannot convert to int: " + ToShortString());
        }

        public override double ToFloat()
        {
            throw new RuntimeLangException("Cannot convert to float: " + ToShortString());
        }

        public override ArrayValue ToArray()
        {
            throw new RuntimeLangException("Cannot convert to array: " + ToShortString());
        }

        public override FunctionValue ToFunction()
        {
            return this;
        }
    }

    public class NativeFunctionValue : FunctionValue
    {
        public Func<Value[], Value> NativeFunc { get; private set; }

        public NativeFunctionValue(Func<Value[], Value> value)
            : base(ValueType.NativeFunction)
        {
            NativeFunc = value;
        }

        public override string ToString()
        {
            return "<NFUNC>";
        }
    }

    public class UserFunctionValue : FunctionValue
    {
        public List<string> ParamList { get; private set; }
        public List<AstStatement> Body { get; private set; }

        public UserFunctionValue(List<string> paramList, List<AstStatement> body)
            : base(ValueType.UserFunction)
        {
            ParamList = paramList;
            Body = body;
        }

        public override string ToString()
        {
            return "<UFUNC>";
        }
    }
}
