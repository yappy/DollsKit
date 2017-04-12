using System;
using System.Collections.Generic;

namespace DollsLang
{
    public class Runtime
    {
        private Dictionary<string, Value> VarTable;
        private Dictionary<string, Func<Value[], Value>> FuncTable;

        public Runtime()
        {
            VarTable = new Dictionary<string, Value>();
            FuncTable = new Dictionary<string, Func<Value[], Value>>();
        }

        public void LoadDefaultFunctions()
        {
            LoadFunction("print", Print);
            LoadFunction("p", Print);
        }

        public void LoadFunction(string funcName, Func<Value[], Value> func)
        {
            FuncTable[funcName] = func;
        }

        public void Execute(AstProgram program)
        {
            foreach (var stat in program.Statements)
            {
                ExecuteStatement(stat);
            }
        }

        private void ExecuteStatement(AstStatement stat)
        {
            switch (stat.Type)
            {
                case NodeType.ASSIGN:
                    {
                        var node = (AstAssign)stat;
                        Assign(node.VariableName, EvalExpression(node.Expression));
                    }
                    break;
                case NodeType.FUNCCALL:
                    {
                        var node = (AstFuncCall)stat;
                        var args = new List<Value>(node.ExpressionList.Count);
                        foreach (var expr in node.ExpressionList)
                        {
                            args.Add(EvalExpression(expr));
                        }
                        CallFunction(node, node.FuncName, args.ToArray());
                    }
                    break;
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
                    throw new FatalLangException();
            }
        }

        private void Assign(string varName, Value value)
        {
            VarTable[varName] = value;
        }

        private Value CallFunction(AstNode at, string funcName, Value[] args)
        {
            Func<Value[], Value> func;
            if (FuncTable.TryGetValue(funcName, out func))
            {
                return FuncTable[funcName](args);
            }
            else
            {
                throw CreateRuntimeError(at, "Function not found: " + funcName);
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
                foreach (var stat in condBody.Body)
                {
                    ExecuteStatement(stat);
                }
            }
        }

        private void executeWhile(AstExpression cond, List<AstStatement> body)
        {
            while (EvalExpression(cond).ToBool())
            {
                foreach (var stat in body)
                {
                    ExecuteStatement(stat);
                }
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
            return new Exception(string.Format("Runtime Error at line {0}, column {1} {2}",
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
