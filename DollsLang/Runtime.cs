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

        private CancellationToken cancel;
        private int callDepth;
        private Dictionary<string, Value> varTable;
        private StringBuilder outputBuffer;
        // Error position info
        private AstNode lastRecord;

        public Runtime(CancellationToken cancel)
        {
            this.cancel = cancel;
            callDepth = 0;
            varTable = new Dictionary<string, Value>();
            outputBuffer = new StringBuilder(OutputSize);
        }

        public void LoadDefaultFunctions()
        {
            LoadFunction("print", libPrint);
            LoadFunction("p", libPrint);
            LoadFunction("for", libFor);
            LoadFunction("foreach", libForEach);
            LoadFunction("size", libSize);
        }

        public void LoadFunction(string funcName, Func<Value[], Value> func)
        {
            assign(funcName, new NativeFunctionValue(func));
        }

        public string Execute(AstProgram program)
        {
            outputBuffer.Clear();
            lastRecord = null;
            try
            {
                executeStatementList(program.Statements);
                return outputBuffer.ToString();
            }
            catch (RuntimeLangException e)
            {
                if (lastRecord != null)
                {
                    e.Line = lastRecord.Line;
                    e.Column = lastRecord.Column;
                }
                throw;
            }
        }

        private Value executeStatementList(List<AstStatement> statList)
        {
            Value result = NilValue.Nil;

            // for 0-length list
            cancel.ThrowIfCancellationRequested();
            foreach (var stat in statList)
            {
                result = executeStatement(stat);
                cancel.ThrowIfCancellationRequested();
            }
            return result;
        }

        private Value executeStatement(AstStatement stat)
        {
            lastRecord = stat;
            switch (stat.Type)
            {
                case NodeType.If:
                    {
                        var node = (AstIf)stat;
                        executeIf(node.CondBobyList);
                        return NilValue.Nil;
                    }
                case NodeType.While:
                    {
                        var node = (AstWhile)stat;
                        executeWhile(node.Cond, node.Body);
                        return NilValue.Nil;
                    }
                default:
                    {
                        var node = (AstExpression)stat;
                        return evalExpression(node);
                    }
            }
        }

        private void executeIf(List<AstIf.CondAndBody> list)
        {
            foreach (var condBody in list)
            {
                // If "if" or "elif", if condition is false, goto next
                if (condBody.Cond != null)
                {
                    bool b = evalExpression(condBody.Cond).ToBool();
                    if (!b)
                    {
                        continue;
                    }
                }
                // Execute block and break
                executeStatementList(condBody.Body);
                break;
            }
        }

        private void executeWhile(AstExpression cond, List<AstStatement> body)
        {
            while (evalExpression(cond).ToBool())
            {
                executeStatementList(body);
            }
        }

        private void assign(string varName, Value value)
        {
            varTable[varName] = value;
        }

        private Value callFunction(FunctionValue funcValue, params Value[] args)
        {
            callDepth++;
            try {
                if (callDepth > DepthMax)
                {
                    throw new RuntimeLangException("Stack overflow");
                }

                switch (funcValue.Type)
                {
                    case ValueType.NativeFunction:
                        return callNativeFunction((NativeFunctionValue)funcValue, args);
                    case ValueType.UserFunction:
                        return callUserFunction((UserFunctionValue)funcValue, args);
                    default:
                        throw new RuntimeLangException("Not a function: " + funcValue.ToString());
                }
            }
            finally
            {
                callDepth--;
            }
        }

        private Value callNativeFunction(NativeFunctionValue funcValue, params Value[] args)
        {
            cancel.ThrowIfCancellationRequested();
            return funcValue.NativeFunc(args);
        }

        private Value callUserFunction(UserFunctionValue funcValue, params Value[] args)
        {
            List<string> paramList = funcValue.ParamList;
            int min = Math.Min(paramList.Count, args.Length);
            for (int i = 0; i < min; i++)
            {
                assign(paramList[i], args[i]);
            }
            return executeStatementList(funcValue.Body);
        }

        private Value readArray(ArrayValue arrayValue, int index)
        {
            List<Value> list = ((ArrayValue)arrayValue).ValueList;
            if (index < 0 || index >= list.Count)
            {
                return NilValue.Nil;
            }
            return list[index];
        }

        private void assignArray(ArrayValue arrayValue, int index, Value value)
        {
            List<Value> list = ((ArrayValue)arrayValue).ValueList;
            // index range check
            if (index < 0 || index >= ArrayMax)
            {
                throw new RuntimeLangException("Invalid array index: " + index);
            }
            // array of array check (against memory attack)
            if (value.Type == ValueType.Array)
            {
                throw new RuntimeLangException("Cannot assign array to array");
            }
            // expand size and fill with nil
            while (list.Count < index + 1)
            {
                list.Add(NilValue.Nil);
            }
            list[index] = value;
        }

        private Value evalExpression(AstExpression expr)
        {
            callDepth++;
            try {
                if (callDepth > DepthMax)
                {
                    throw new RuntimeLangException("Stack overflow");
                }
                lastRecord = expr;

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
                            if (varTable.TryGetValue(node.Name, out value))
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
                            return evalOperation(node);
                        }
                    case NodeType.Assign:
                        {
                            var node = (AstAssign)expr;
                            Value value = evalExpression(node.Expression);
                            assign(node.VariableName, value);
                            return value;
                        }
                    case NodeType.AssignArray:
                        {
                            var node = (AstAssignArray)expr;
                            ArrayValue arrayValue = evalExpression(node.Array).ToArray();
                            int index = evalExpression(node.Index).ToInt();
                            Value value = evalExpression(node.Expression);
                            assignArray(arrayValue, index, value);
                            return value;
                        }
                    case NodeType.ReadArray:
                        {
                            var node = (AstReadArray)expr;
                            ArrayValue arrayValue = evalExpression(node.Array).ToArray();
                            int index = evalExpression(node.Index).ToInt();
                            return readArray(arrayValue, index);
                        }
                    case NodeType.FunctionCall:
                        {
                            var node = (AstFunctionCall)expr;
                            FunctionValue funcValue = evalExpression(node.Func).ToFunction();
                            var args = new List<Value>(node.ExpressionList.Count);
                            foreach (var arg in node.ExpressionList)
                            {
                                args.Add(evalExpression(arg));
                            }
                            return callFunction(funcValue, args.ToArray());
                        }
                    case NodeType.ConstructArray:
                        {
                            var node = (AstConstructArray)expr;
                            var valueList = new List<Value>(node.ExpressionList.Count);
                            var value = new ArrayValue(valueList);
                            int index = 0;
                            foreach (var elem in node.ExpressionList)
                            {
                                assignArray(value, index, evalExpression(elem));
                                index++;
                            }
                            return value;
                        }
                    default:
                        throw new FatalLangException();
                }
            }
            finally
            {
                callDepth--;
            }
        }

        private Value evalOperation(AstOperation node)
        {
            lastRecord = node;

            var args = new Value[node.Operands.Length];
            args[0] = evalExpression(node.Operands[0]);

            lastRecord = node;

            // short circuit
            switch (node.Operaton)
            {
                case OperationType.And:
                    if (!args[0].ToBool())
                    {
                        return args[0];
                    }
                    break;
                case OperationType.Or:
                    if (args[0].ToBool())
                    {
                        return args[0];
                    }
                    break;
            }

            for (int i = 1; i < args.Length; i++)
            {
                args[i] = evalExpression(node.Operands[i]);
            }

            lastRecord = node;
            switch (node.Operaton)
            {
                case OperationType.Negative:
                    switch (args[0].Type)
                    {
                        case ValueType.Int:
                            return new IntValue(-args[0].ToInt());
                        case ValueType.Float:
                            return new FloatValue(-args[0].ToFloat());
                        default:
                            throw new RuntimeLangException(
                                $"Cannot apply {node.Operaton} operator: {args[0].Type}");
                    }
                case OperationType.Not:
                    return BoolValue.Of(!args[0].ToBool());

                case OperationType.Add:
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
                            $"Cannot apply + operator: {args[0].Type}, {args[1].Type}");
                    }
                case OperationType.Sub:
                case OperationType.Mul:
                case OperationType.Div:
                case OperationType.Mod:
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
                            case OperationType.Sub:
                                return new FloatValue(lh - rh);
                            case OperationType.Mul:
                                return new FloatValue(lh * rh);
                            case OperationType.Div:
                                return new FloatValue(lh / rh);
                            case OperationType.Mod:
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
                            case OperationType.Sub:
                                return new IntValue(lh - rh);
                            case OperationType.Mul:
                                return new IntValue(lh * rh);
                            case OperationType.Div:
                                if (rh == 0)
                                {
                                    throw new RuntimeLangException("Divide by 0");
                                }
                                return new IntValue(lh / rh);
                            case OperationType.Mod:
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
                            $"Cannot apply {node.Operaton} operator: " +
                            $"{args[0].Type}, {args[1].Type}");
                    }
                // simply returns rh (short circuit passed)
                case OperationType.And:
                    return args[1];
                case OperationType.Or:
                    return args[1];
            }
            throw new FatalLangException();
        }

        private Value getParam(Value[] args, int index)
        {
            if (index >= args.Length)
            {
                throw new RuntimeLangException(
                    $"Parameter #{index + 1} is required");
            }
            return args[index];
        }

        private Value libPrint(Value[] args)
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

        private Value libFor(Value[] args)
        {
            int start = getParam(args, 0).ToInt();
            int end = getParam(args, 1).ToInt();
            FunctionValue func = getParam(args, 2).ToFunction();

            Value[] callArgs = new Value[1];
            for (int i = start; i <= end; i++)
            {
                callArgs[0] = new IntValue(i);
                callFunction(func, callArgs);
            }

            return NilValue.Nil;
        }

        private Value libForEach(Value[] args)
        {
            ArrayValue array = getParam(args, 0).ToArray();
            List<Value> list = array.ValueList;
            FunctionValue func = getParam(args, 1).ToFunction();

            Value[] callArgs = new Value[2];
            for (int i = 0; i < list.Count; i++)
            {
                callArgs[0] = list[i];
                callArgs[1] = new IntValue(i);
                callFunction(func, callArgs);
            }

            return NilValue.Nil;
        }

        private Value libSize(Value[] args)
        {
            ArrayValue array = getParam(args, 0).ToArray();

            return new IntValue(array.ValueList.Count);
        }
    }
}
