using System;
using System.Collections.Generic;
using System.Threading;

namespace DollsLang
{
    public class Runtime
    {
        private Dictionary<string, Value> VarTable;
        private CancellationToken Cancel;

        public Runtime(CancellationToken cancel)
        {
            VarTable = new Dictionary<string, Value>();
            Cancel = cancel;
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

        public void Execute(AstProgram program)
        {
            ExecuteStatementList(program.Statements);
        }

        private void ExecuteStatementList(List<AstStatement> statList)
        {
            // for 0-length list
            Cancel.ThrowIfCancellationRequested();
            foreach (var stat in statList)
            {
                Cancel.ThrowIfCancellationRequested();
                ExecuteStatement(stat);
            }
        }

        private void ExecuteStatement(AstStatement stat)
        {
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
                        executeWhile(node.Cond, node.Body);
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

        private Value CallFunction(AstNode at, Value funcValue, Value[] args)
        {
            if (funcValue.Type != ValueType.FUNCTION)
            {
                throw CreateRuntimeError(at, "Not a function: " + funcValue.ToString());
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

        private void executeWhile(AstExpression cond, List<AstStatement> body)
        {
            while (EvalExpression(cond).ToBool())
            {
                ExecuteStatementList(body);
            }
        }

        private Value EvalExpression(AstExpression expr)
        {
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
                        return CallFunction(node, funcValue, args.ToArray());
                    }
                default:
                    throw new FatalLangException();
            }
        }

        private Value EvalOperation(AstOperation node)
        {
            var args = new Value[node.Operands.Length];
            args[0] = EvalExpression(node.Operands[0]);

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
                            throw CreateRuntimeError(node,
                                string.Format("Cannot apply {0} operator: {1}",
                                    node.Operaton, args[0].Type));
                    }
                case OperationType.NOT:
                    return BoolValue.Of(!args[0].ToBool());

                case OperationType.ADD:
                    if (args[0].Type == ValueType.STRING || args[1].Type == ValueType.STRING)
                    {
                        return new StringValue(args[0].ToString() + args[1].ToString());
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
                        throw CreateRuntimeError(node, string.Format(
                            "Cannot apply + operator: {0}, {1}",
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
                                    throw CreateRuntimeError(node, "Divide by 0");
                                }
                                return new IntValue(lh / rh);
                            case OperationType.MOD:
                                if (rh == 0)
                                {
                                    throw CreateRuntimeError(node, "Divide by 0");
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
                        throw CreateRuntimeError(node, string.Format(
                            "Cannot apply {0} operator: {1}, {2}",
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

        private Exception CreateRuntimeError(AstNode at, string message = "")
        {
            return new RuntimeLangException(string.Format("Runtime Error at line {0}, column {1} {2}",
                at.Line, at.Column, message));
        }

        private Value Print(Value[] args)
        {
            bool first = true;
            foreach (var value in args)
            {
                if (!first)
                {
                    Console.Write(' ');
                }
                first = false;
                Console.Write(value.ToString());
            }
            Console.WriteLine();

            return NilValue.Nil;
        }
    }
}
