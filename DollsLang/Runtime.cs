using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading;

namespace DollsLang
{
    public partial class Runtime
    {
        private static readonly int OutputSize = 140;
        private static readonly int StringMax = 256;
        private static readonly int ArrayMax = 1024;
        private static readonly int DepthMax = 1024;

        private CancellationToken cancel;
        private int callDepth;
        private Dictionary<string, Value> varTable;
        // Error position info
        private AstNode lastRecord;

        public Runtime(CancellationToken cancel)
        {
            this.cancel = cancel;
            callDepth = 0;
            varTable = new Dictionary<string, Value>();
            outputBuffer = new StringBuilder(OutputSize);
        }

        public void LoadDefaultLibrary()
        {
            LoadDefaultVariablesInternal();
            LoadDefaultFunctionsInternal();
        }

        public void LoadIntVariable(string name, int value)
        {
            Assign(name, new IntValue(value));
        }

        public void LoadFloatVariable(string name, double value)
        {
            Assign(name, new FloatValue(value));
        }

        public void LoadFunction(string funcName, Func<Value[], Value> func)
        {
            Assign(funcName, new NativeFunctionValue(func));
        }

        public string Execute(AstProgram program)
        {
            InitializeRuntime();
            lastRecord = null;
            try
            {
                ExecuteStatementList(program.Statements);
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

        private Value ExecuteStatementList(List<AstStatement> statList)
        {
            Value result = NilValue.Nil;

            // for 0-length list
            cancel.ThrowIfCancellationRequested();
            foreach (var stat in statList)
            {
                result = ExecuteStatement(stat);
                cancel.ThrowIfCancellationRequested();
            }
            return result;
        }

        private Value ExecuteStatement(AstStatement stat)
        {
            lastRecord = stat;
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
            varTable[varName] = value;
        }

        private Value CallFunction(FunctionValue funcValue, params Value[] args)
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
                        return CallNativeFunction((NativeFunctionValue)funcValue, args);
                    case ValueType.UserFunction:
                        return CallUserFunction((UserFunctionValue)funcValue, args);
                    default:
                        throw new RuntimeLangException("Not a function: " + funcValue.ToString());
                }
            }
            finally
            {
                callDepth--;
            }
        }

        private Value CallNativeFunction(NativeFunctionValue funcValue, params Value[] args)
        {
            cancel.ThrowIfCancellationRequested();
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

        private Value ReadArray(ArrayValue arrayValue, int index)
        {
            List<Value> list = ((ArrayValue)arrayValue).ValueList;
            if (index < 0 || index >= list.Count)
            {
                return NilValue.Nil;
            }
            return list[index];
        }

        private void AssignArray(ArrayValue arrayValue, int index, Value value)
        {
            List<Value> list = ((ArrayValue)arrayValue).ValueList;
            // index range check
            if (index < 0 || index >= ArrayMax)
            {
                throw new RuntimeLangException($"Invalid array index: {index}");
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

        private ArrayValue ConcatArray(ArrayValue lh, ArrayValue rh)
        {
            List<Value> llist = lh.ValueList;
            List<Value> rlist = rh.ValueList;
            if (llist.Count + rlist.Count > ArrayMax)
            {
                throw new RuntimeLangException("Array size over");
            }
            List<Value> resultList = llist.Concat(rlist).ToList();
            return new ArrayValue(resultList);
        }

        private Value EvalExpression(AstExpression expr)
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
#pragma warning disable IDE0018 // インライン変数宣言
                            Value value;
#pragma warning restore IDE0018 // インライン変数宣言
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
                            ArrayValue arrayValue = EvalExpression(node.Array).ToArray();
                            int index = EvalExpression(node.Index).ToInt();
                            Value value = EvalExpression(node.Expression);
                            AssignArray(arrayValue, index, value);
                            return value;
                        }
                    case NodeType.ReadArray:
                        {
                            var node = (AstReadArray)expr;
                            ArrayValue arrayValue = EvalExpression(node.Array).ToArray();
                            int index = EvalExpression(node.Index).ToInt();
                            return ReadArray(arrayValue, index);
                        }
                    case NodeType.FunctionCall:
                        {
                            var node = (AstFunctionCall)expr;
                            FunctionValue funcValue = EvalExpression(node.Func).ToFunction();
                            var args = new List<Value>(node.ExpressionList.Count);
                            foreach (var arg in node.ExpressionList)
                            {
                                args.Add(EvalExpression(arg));
                            }
                            lastRecord = expr;
                            return CallFunction(funcValue, args.ToArray());
                        }
                    case NodeType.ConstructArray:
                        {
                            var node = (AstConstructArray)expr;
                            var valueList = new List<Value>(node.ExpressionList.Count);
                            var value = new ArrayValue(valueList);
                            int index = 0;
                            foreach (var elem in node.ExpressionList)
                            {
                                AssignArray(value, index, EvalExpression(elem));
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

        private Value EvalOperation(AstOperation node)
        {
            lastRecord = node;

            var args = new Value[node.Operands.Length];
            args[0] = EvalExpression(node.Operands[0]);

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
                args[i] = EvalExpression(node.Operands[i]);
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
                    if (args[0].Type == ValueType.Array && args[1].Type == ValueType.Array)
                    {
                        return ConcatArray(args[0].ToArray(), args[1].ToArray());
                    }
                    else if (args[0].Type == ValueType.String || args[1].Type == ValueType.String)
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
    }
}
