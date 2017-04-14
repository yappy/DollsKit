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

        private CancellationToken Cancel;
        private Dictionary<string, Value> VarTable;
        private StringBuilder OutputBuffer;
        // Error position info
        private AstNode LastRecord;

        public Runtime(CancellationToken cancel)
        {
            VarTable = new Dictionary<string, Value>();
            Cancel = cancel;
            OutputBuffer = new StringBuilder(OutputSize);
        }

        public void LoadDefaultFunctions()
        {
            LoadFunction("print", Print);
            LoadFunction("p", Print);
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

        private void ExecuteStatementList(List<AstStatement> statList)
        {
            // for 0-length list
            Cancel.ThrowIfCancellationRequested();
            foreach (var stat in statList)
            {
                ExecuteStatement(stat);
                Cancel.ThrowIfCancellationRequested();
            }
        }

        private void ExecuteStatement(AstStatement stat)
        {
            LastRecord = stat;
            switch (stat.Type)
            {
                case NodeType.IF:
                    {
                        var node = (AstIf)stat;
                        ExecuteIf(node.CondBobyList);
                    }
                    break;
                case NodeType.WHILE:
                    {
                        var node = (AstWhile)stat;
                        ExecuteWhile(node.Cond, node.Body);
                    }
                    break;
                default:
                    {
                        var node = (AstExpression)stat;
                        EvalExpression(node);
                    }
                    break;
            }
        }

        private void Assign(string varName, Value value)
        {
            VarTable[varName] = value;
        }

        private Value CallFunction(Value funcValue, Value[] args)
        {
            if (funcValue.Type != ValueType.FUNCTION)
            {
                throw new RuntimeLangException("Not a function: " + funcValue.ToString());
            }
            return ((FunctionValue)funcValue).Call(args);
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

        private Value EvalExpression(AstExpression expr)
        {
            LastRecord = expr;
            switch (expr.Type)
            {
                case NodeType.CONSTANT:
                    {
                        var node = (AstConstant)expr;
                        return node.Value;
                    }
                case NodeType.VARIABLE:
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
                case NodeType.OPERATION:
                    {
                        var node = (AstOperation)expr;
                        return EvalOperation(node);
                    }
                case NodeType.ASSIGN:
                    {
                        var node = (AstAssign)expr;
                        Value value = EvalExpression(node.Expression);
                        Assign(node.VariableName, value);
                        return value;
                    }
                case NodeType.FUNCCALL:
                    {
                        var node = (AstFuncCall)expr;
                        var funcValue = EvalExpression(node.Func);
                        var args = new List<Value>(node.ExpressionList.Count);
                        foreach (var arg in node.ExpressionList)
                        {
                            args.Add(EvalExpression(arg));
                        }
                        return CallFunction(funcValue, args.ToArray());
                    }
                default:
                    throw new FatalLangException();
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
                        case ValueType.INT:
                            return new IntValue(-args[0].ToInt());
                        case ValueType.FLOAT:
                            return new FloatValue(-args[0].ToFloat());
                        default:
                            throw new RuntimeLangException(
                                string.Format("Cannot apply {0} operator: {1}",
                                    node.Operaton, args[0].Type));
                    }
                case OperationType.NOT:
                    return BoolValue.Of(!args[0].ToBool());

                case OperationType.ADD:
                    if (args[0].Type == ValueType.STRING || args[1].Type == ValueType.STRING)
                    {
                        var result = args[0].ToString() + args[1].ToString();
                        if (result.Length > StringMax)
                        {
                            throw new RuntimeLangException("String size over");
                        }
                        return new StringValue(result);
                    }
                    else if (args[0].Type == ValueType.FLOAT || args[1].Type == ValueType.FLOAT)
                    {
                        return new FloatValue(args[0].ToFloat() + args[1].ToFloat());
                    }
                    else if (args[0].Type == ValueType.INT || args[1].Type == ValueType.INT)
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
                    if (args[0].Type == ValueType.FLOAT || args[1].Type == ValueType.FLOAT)
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
                    else if (args[0].Type == ValueType.INT || args[1].Type == ValueType.INT)
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
    }
}
