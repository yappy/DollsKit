using System;
using System.Collections.Generic;
using System.Text;
using System.Threading;

namespace DollsLang
{
    public class Runtime
    {
        private static readonly int OutputSize = 140;
        private static readonly int StringMax = 256;
        private static readonly int ArrayMax = 1024;
        private static readonly int DepthMax = 1024;

        private CancellationToken Cancel;
        private int CallDepth;
        private Dictionary<string, Value> VarTable;
        private StringBuilder OutputBuffer;
        // Error position info
        private AstNode LastRecord;

        public Runtime(CancellationToken cancel)
        {
            Cancel = cancel;
            CallDepth = 0;
            VarTable = new Dictionary<string, Value>();
            OutputBuffer = new StringBuilder(OutputSize);
        }

        public void LoadDefaultFunctions()
        {
            LoadFunction("print", Print);
            LoadFunction("p", Print);
            LoadFunction("for", For);
        }

        public void LoadFunction(string funcName, Func<Value[], Value> func)
        {
            VarTable[funcName] = new NativeFunctionValue(func);
        }

        public string Execute(AstProgram program)
        {
            OutputBuffer.Clear();
            LastRecord = null;
            try
            {
                ExecuteStatementList(program.Statements);
                return OutputBuffer.ToString();
            }
            catch (RuntimeLangException e)
            {
                if (LastRecord != null)
                {
                    e.Line = LastRecord.Line;
                    e.Column = LastRecord.Column;
                }
                throw;
            }
        }

        private Value ExecuteStatementList(List<AstStatement> statList)
        {
            Value result = NilValue.Nil;

            // for 0-length list
            Cancel.ThrowIfCancellationRequested();
            foreach (var stat in statList)
            {
                result = ExecuteStatement(stat);
                Cancel.ThrowIfCancellationRequested();
            }
            return result;
        }

        private Value ExecuteStatement(AstStatement stat)
        {
            LastRecord = stat;
            switch (stat.Type)
            {
                case NodeType.If:
                    {
                        var node = (AstIf)stat;
                        ExecuteIf(node.CondBobyList);
                        return NilValue.Nil;
                    }
                case NodeType.While:
                    {
                        var node = (AstWhile)stat;
                        ExecuteWhile(node.Cond, node.Body);
                        return NilValue.Nil;
                    }
                default:
                    {
                        var node = (AstExpression)stat;
                        return EvalExpression(node);
                    }
            }
        }

        private void ExecuteIf(List<AstIf.CondAndBody> list)
        {
            foreach (var condBody in list)
            {
                // If "if" or "elif", if condition is false, goto next
                if (condBody.Cond != null)
                {
                    bool b = EvalExpression(condBody.Cond).ToBool();
                    if (!b)
                    {
                        continue;
                    }
                }
                // Execute block and break
                ExecuteStatementList(condBody.Body);
                break;
            }
        }

        private void ExecuteWhile(AstExpression cond, List<AstStatement> body)
        {
            while (EvalExpression(cond).ToBool())
            {
                ExecuteStatementList(body);
            }
        }

        private void Assign(string varName, Value value)
        {
            VarTable[varName] = value;
        }

        private Value CallFunction(Value funcValue, params Value[] args)
        {
            CallDepth++;
            try {
                if (CallDepth > DepthMax)
                {
                    throw new RuntimeLangException("Stack overflow");
                }

                switch (funcValue.Type)
                {
                    case ValueType.NativeFunction:
                        return CallNativeFunction((NativeFunctionValue)funcValue, args);
                    case ValueType.UserFunction:
                        return CallUserFunction((UserFunctionValue)funcValue, args);
                    default:
                        throw new RuntimeLangException("Not a function: " + funcValue.ToString());
                }
            }
            finally
            {
                CallDepth--;
            }
        }

        private Value CallNativeFunction(NativeFunctionValue funcValue, params Value[] args)
        {
            return funcValue.NativeFunc(args);
        }

        private Value CallUserFunction(UserFunctionValue funcValue, params Value[] args)
        {
            List<string> paramList = funcValue.ParamList;
            int min = Math.Min(paramList.Count, args.Length);
            for (int i = 0; i < min; i++)
            {
                Assign(paramList[i], args[i]);
            }
            return ExecuteStatementList(funcValue.Body);
        }

        private Value ReadArray(Value arrayValue, Value indexValue)
        {
            if (arrayValue.Type != ValueType.Array)
            {
                throw new RuntimeLangException("Not an array: " + arrayValue.ToString());
            }
            List<Value> list = ((ArrayValue)arrayValue).ValueList;
            int index = indexValue.ToInt();
            if (index < 0 || index >= list.Count)
            {
                return NilValue.Nil;
            }
            return list[index];
        }

        private void AssignArray(Value arrayValue, Value indexValue, Value value)
        {
            if (arrayValue.Type != ValueType.Array)
            {
                throw new RuntimeLangException("Not an array: " + arrayValue.ToString());
            }
            List<Value> list = ((ArrayValue)arrayValue).ValueList;
            int index = indexValue.ToInt();
            if (index < 0 || index >= ArrayMax)
            {
                throw new RuntimeLangException("Invalid array index: " + index);
            }
            while (list.Count < index + 1)
            {
                list.Add(NilValue.Nil);
            }
            list[index] = value;
        }

        private Value EvalExpression(AstExpression expr)
        {
            CallDepth++;
            try {
                if (CallDepth > DepthMax)
                {
                    throw new RuntimeLangException("Stack overflow");
                }
                LastRecord = expr;

                switch (expr.Type)
                {
                    case NodeType.Constant:
                        {
                            var node = (AstConstant)expr;
                            return node.Value;
                        }
                    case NodeType.Variable:
                        {
                            var node = (AstVariable)expr;
                            Value value;
                            if (VarTable.TryGetValue(node.Name, out value))
                            {
                                return value;
                            }
                            else
                            {
                                return NilValue.Nil;
                            }
                        }
                    case NodeType.Operation:
                        {
                            var node = (AstOperation)expr;
                            return EvalOperation(node);
                        }
                    case NodeType.Assign:
                        {
                            var node = (AstAssign)expr;
                            Value value = EvalExpression(node.Expression);
                            Assign(node.VariableName, value);
                            return value;
                        }
                    case NodeType.AssignArray:
                        {
                            var node = (AstAssignArray)expr;
                            Value arrayValue = EvalExpression(node.Array);
                            Value indexValue = EvalExpression(node.Index);
                            Value value = EvalExpression(node.Expression);
                            AssignArray(arrayValue, indexValue, value);
                            return value;
                        }
                    case NodeType.ReadArray:
                        {
                            var node = (AstReadArray)expr;
                            Value arrayValue = EvalExpression(node.Array);
                            Value indexValue = EvalExpression(node.Index);
                            return ReadArray(arrayValue, indexValue);
                        }
                    case NodeType.FunctionCall:
                        {
                            var node = (AstFunctionCall)expr;
                            var funcValue = EvalExpression(node.Func);
                            var args = new List<Value>(node.ExpressionList.Count);
                            foreach (var arg in node.ExpressionList)
                            {
                                args.Add(EvalExpression(arg));
                            }
                            return CallFunction(funcValue, args.ToArray());
                        }
                    case NodeType.ConstructArray:
                        {
                            var node = (AstConstructArray)expr;
                            var valueList = new List<Value>();
                            foreach (var elem in node.ExpressionList)
                            {
                                valueList.Add(EvalExpression(elem));
                            }
                            return new ArrayValue(valueList);
                        }
                    default:
                        throw new FatalLangException();
                }
            }
            finally
            {
                CallDepth--;
            }
        }

        private Value EvalOperation(AstOperation node)
        {
            LastRecord = node;

            var args = new Value[node.Operands.Length];
            args[0] = EvalExpression(node.Operands[0]);

            LastRecord = node;

            // short circuit
            switch (node.Operaton)
            {
                case OperationType.AND:
                    if (!args[0].ToBool())
                    {
                        return args[0];
                    }
                    break;
                case OperationType.OR:
                    if (args[0].ToBool())
                    {
                        return args[0];
                    }
                    break;
            }

            for (int i = 1; i < args.Length; i++)
            {
                args[i] = EvalExpression(node.Operands[i]);
            }

            LastRecord = node;
            switch (node.Operaton)
            {
                case OperationType.NEGATIVE:
                    switch (args[0].Type)
                    {
                        case ValueType.Int:
                            return new IntValue(-args[0].ToInt());
                        case ValueType.Float:
                            return new FloatValue(-args[0].ToFloat());
                        default:
                            throw new RuntimeLangException(
                                string.Format("Cannot apply {0} operator: {1}",
                                    node.Operaton, args[0].Type));
                    }
                case OperationType.NOT:
                    return BoolValue.Of(!args[0].ToBool());

                case OperationType.ADD:
                    if (args[0].Type == ValueType.String || args[1].Type == ValueType.String)
                    {
                        var result = args[0].ToString() + args[1].ToString();
                        if (result.Length > StringMax)
                        {
                            throw new RuntimeLangException("String size over");
                        }
                        return new StringValue(result);
                    }
                    else if (args[0].Type == ValueType.Float || args[1].Type == ValueType.Float)
                    {
                        return new FloatValue(args[0].ToFloat() + args[1].ToFloat());
                    }
                    else if (args[0].Type == ValueType.Int || args[1].Type == ValueType.Int)
                    {
                        return new IntValue(args[0].ToInt() + args[1].ToInt());
                    }
                    else
                    {
                        throw new RuntimeLangException(
                            string.Format("Cannot apply + operator: {0}, {1}",
                                args[0].Type, args[1].Type));
                    }
                case OperationType.SUB:
                case OperationType.MUL:
                case OperationType.DIV:
                case OperationType.MOD:
                case OperationType.LT:
                case OperationType.LE:
                case OperationType.GT:
                case OperationType.GE:
                case OperationType.EQ:
                case OperationType.NE:
                    if (args[0].Type == ValueType.Float || args[1].Type == ValueType.Float)
                    {
                        double lh = args[0].ToFloat();
                        double rh = args[1].ToFloat();
                        switch (node.Operaton)
                        {
                            case OperationType.SUB:
                                return new FloatValue(lh - rh);
                            case OperationType.MUL:
                                return new FloatValue(lh * rh);
                            case OperationType.DIV:
                                return new FloatValue(lh / rh);
                            case OperationType.MOD:
                                return new FloatValue(lh % rh);
                            case OperationType.LT:
                                return BoolValue.Of(lh < rh);
                            case OperationType.LE:
                                return BoolValue.Of(lh <= rh);
                            case OperationType.GT:
                                return BoolValue.Of(lh > rh);
                            case OperationType.GE:
                                return BoolValue.Of(lh >= rh);
                            case OperationType.EQ:
                                return BoolValue.Of(lh == rh);
                            case OperationType.NE:
                                return BoolValue.Of(lh != rh);
                            default:
                                throw new FatalLangException();
                        }

                    }
                    else if (args[0].Type == ValueType.Int || args[1].Type == ValueType.Int)
                    {
                        int lh = args[0].ToInt();
                        int rh = args[1].ToInt();
                        switch (node.Operaton)
                        {
                            case OperationType.SUB:
                                return new IntValue(lh - rh);
                            case OperationType.MUL:
                                return new IntValue(lh * rh);
                            case OperationType.DIV:
                                if (rh == 0)
                                {
                                    throw new RuntimeLangException("Divide by 0");
                                }
                                return new IntValue(lh / rh);
                            case OperationType.MOD:
                                if (rh == 0)
                                {
                                    throw new RuntimeLangException("Divide by 0");
                                }
                                return new IntValue(lh % rh);
                            case OperationType.LT:
                                return BoolValue.Of(lh < rh);
                            case OperationType.LE:
                                return BoolValue.Of(lh <= rh);
                            case OperationType.GT:
                                return BoolValue.Of(lh > rh);
                            case OperationType.GE:
                                return BoolValue.Of(lh >= rh);
                            case OperationType.EQ:
                                return BoolValue.Of(lh == rh);
                            case OperationType.NE:
                                return BoolValue.Of(lh != rh);
                            default:
                                throw new FatalLangException();
                        }
                    }
                    else
                    {
                        throw new RuntimeLangException(
                            string.Format("Cannot apply {0} operator: {1}, {2}",
                                node.Operaton, args[0].Type, args[1].Type));
                    }
                // simply returns rh (short circuit passed)
                case OperationType.AND:
                    return args[1];
                case OperationType.OR:
                    return args[1];
            }
            throw new FatalLangException();
        }

        private Value GetParam(Value[] args, int index)
        {
            if (index >= args.Length)
            {
                throw new RuntimeLangException(
                    string.Format("Parameter #{0} is required", index + 1));
            }
            return args[index];
        }

        private Value Print(Value[] args)
        {
            bool first = true;
            foreach (var value in args)
            {
                if (!first)
                {
                    OutputBuffer.Append(' ');
                }
                first = false;
                OutputBuffer.Append(value.ToString());
            }
            OutputBuffer.Append('\n');
            OutputBuffer.Length = Math.Min(OutputBuffer.Length, OutputSize);

            return NilValue.Nil;
        }

        private Value For(Value[] args)
        {
            int start = GetParam(args, 0).ToInt();
            int end = GetParam(args, 1).ToInt();
            Value func = GetParam(args, 2);

            Value[] callArgs = new Value[1];
            for (int i = start; i <= end; i++)
            {
                callArgs[0] = new IntValue(i);
                CallFunction(func, callArgs);
            }

            return NilValue.Nil;
        }
    }
}
