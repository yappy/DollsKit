using System.Collections.Generic;
using System.Text;

namespace DollsLang
{
    public enum ValueType
    {
        NIL, BOOL, INT, FLOAT, STRING,
    }

    public abstract class Value
    {
        public ValueType Type { get; private set; }

        protected Value(ValueType type)
        {
            Type = type;
        }

        public abstract bool ToBool();
        public abstract int ToInt();
        public abstract double ToFloat();
    }

    public class NilValue : Value
    {
        private static readonly NilValue obj = new NilValue();
        public static NilValue Nil
        {
            get
            {
                return obj;
            }
        }

        private NilValue()
            : base(ValueType.NIL)
        { }

        public override bool ToBool()
        {
            return false;
        }

        public override int ToInt()
        {
            throw new RuntimeLangException("nil cannot be converted to int");
        }

        public override double ToFloat()
        {
            throw new RuntimeLangException("nil cannot be converted to float");
        }

        public override string ToString()
        {
            return "nil";
        }
    }

    public class BoolValue : Value
    {
        public bool RawValue { get; private set; }

        private static readonly BoolValue tobj = new BoolValue(true);
        private static readonly BoolValue fobj = new BoolValue(false);

        public static BoolValue True
        {
            get
            {
                return tobj;
            }
        }
        public static BoolValue False
        {
            get
            {
                return fobj;
            }
        }

        public static Value Of(bool b)
        {
            return b ? True : False;
        }

        private BoolValue(bool b)
            : base(ValueType.BOOL)
        {
            RawValue = b;
        }

        public override bool ToBool()
        {
            return RawValue;
        }

        public override int ToInt()
        {
            throw new RuntimeLangException("bool cannot be converted to int");
        }

        public override double ToFloat()
        {
            throw new RuntimeLangException("bool cannot be converted to float");
        }

        public override string ToString()
        {
            return RawValue ? "true" : "false";
        }
    }

    public class IntValue : Value
    {
        public int RawValue { get; private set; }

        public IntValue(int value)
            : base(ValueType.INT)
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
    }

    public class FloatValue : Value
    {
        public double RawValue { get; private set; }

        public FloatValue(double value)
            : base(ValueType.FLOAT)
        {
            RawValue = value;
        }

        public override bool ToBool()
        {
            return true;
        }

        public override int ToInt()
        {
            return (int)RawValue;
        }

        public override double ToFloat()
        {
            return RawValue;
        }

        public override string ToString()
        {
            return RawValue.ToString();
        }
    }

    public class StringValue : Value
    {
        public string RawValue { get; private set; }

        public StringValue(string value)
            : base(ValueType.STRING)
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
                throw new RuntimeLangException("Cannot convert to int: " + RawValue);
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
                throw new RuntimeLangException("Cannot convert to float: " + RawValue);
            }
        }

        public override string ToString()
        {
            return RawValue;
        }
    }
}
