using System.Collections.Generic;
using System.Text;

namespace DollsLang
{
    public enum NodeType
    {
        PROGRAM,
        ASSIGN,
        FUNCCALL,
        IF, WHILE,
        OPERATION, VARIABLE, CONSTANT,
    }

    public enum OperationType
    {
        NEGATIVE, NOT,
        MUL, DIV, MOD,
        ADD, SUB,
        LT, LE, GT, GE,
        EQ, NE,
        AND,
        OR,
    }

    public abstract class AstNode
    {
        public NodeType Type { get; private set; }
        public int Line { get; private set; }
        public int Column { get; private set; }

        protected AstNode(Token from, NodeType type)
        {
            Type = type;
            if (from != null)
            {
                Line = from.Line;
                Column = from.Column;
            }
            else
            {
                Line = 0;
                Column = 0;
            }
        }

        public void Print(StringBuilder buf, int depth)
        {
            for (int i = 0; i < depth; i++)
            {
                buf.Append('\t');
            }
            buf.AppendLine(ToString());
            PrintChildren(buf, depth + 1);
        }

        public override string ToString()
        {
            return Type.ToString();
        }

        protected virtual void PrintChildren(StringBuilder buf, int depth) { }
    }

    public abstract class AstStatement : AstNode
    {
        protected AstStatement(Token from, NodeType type)
            : base(from, type)
        { }
    }

    public abstract class AstExpression : AstStatement
    {
        protected AstExpression(Token from, NodeType type)
            : base(from, type)
        { }
    }

    public class AstProgram : AstNode
    {
        public List<AstStatement> Statements { get; private set; }

        public AstProgram(Token from, List<AstStatement> statements)
            : base(from, NodeType.PROGRAM)
        {
            Statements = statements;
        }

        protected override void PrintChildren(StringBuilder buf, int depth)
        {
            foreach (var child in Statements)
            {
                child.Print(buf, depth);
            }
        }
    }

    class AstIf : AstStatement
    {
        public class CondAndBody
        {
            // null if "else"
            public AstExpression Cond { get; private set; }
            public List<AstStatement> Body { get; private set; }

            public CondAndBody(AstExpression cond, List<AstStatement> body)
            {
                Cond = cond;
                Body = body;
            }
        }

        public List<CondAndBody> CondBobyList { get; private set; }

        public AstIf(Token from, List<CondAndBody> condBodyList)
            : base(from, NodeType.IF)
        {
            CondBobyList = condBodyList;
        }

        protected override void PrintChildren(StringBuilder buf, int depth)
        {
            foreach (var child in CondBobyList)
            {
                if (child.Cond != null)
                {
                    child.Cond.Print(buf, depth);
                }
                else
                {
                    for (int i = 0; i < depth; i++)
                    {
                        buf.Append('\t');
                    }
                    buf.AppendLine("ELSE");
                }
                foreach (var stat in child.Body)
                {
                    stat.Print(buf, depth);
                }
            }
        }
    }

    class AstWhile : AstStatement
    {
        public AstExpression Cond { get; private set; }
        public List<AstStatement> Body { get; private set; }

        public AstWhile(Token from, AstExpression cond, List<AstStatement> body)
            : base(from, NodeType.WHILE)
        {
            Cond = cond;
            Body = body;
        }

        protected override void PrintChildren(StringBuilder buf, int depth)
        {
            Cond.Print(buf, depth);
            foreach (var stat in Body)
            {
                stat.Print(buf, depth);
            }
        }
    }

    class AstOperation : AstExpression
    {
        public OperationType Operaton { get; private set; }
        public AstExpression[] Operands;

        public AstOperation(Token from, OperationType operation, params AstExpression[] operands)
            : base(from, NodeType.OPERATION)
        {
            Operaton = operation;
            Operands = operands;
        }

        public override string ToString()
        {
            return base.ToString() + " " + Operaton;
        }

        protected override void PrintChildren(StringBuilder buf, int depth)
        {
            foreach (var child in Operands)
            {
                child.Print(buf, depth);
            }
        }
    }

    class AstAssign : AstExpression
    {
        public string VariableName { get; private set; }
        public AstExpression Expression { get; private set; }

        public AstAssign(Token from, string variableName, AstExpression expression)
            : base(from, NodeType.ASSIGN)
        {
            VariableName = variableName;
            Expression = expression;
        }

        public override string ToString()
        {
            return base.ToString() + " " + VariableName;
        }

        protected override void PrintChildren(StringBuilder buf, int depth)
        {
            Expression.Print(buf, depth);
        }
    }

    class AstFuncCall : AstExpression
    {
        public AstExpression Func { get; private set; }
        public List<AstExpression> ExpressionList { get; private set; }

        public AstFuncCall(Token from, AstExpression func, List<AstExpression> expressionList)
            : base(from, NodeType.FUNCCALL)
        {
            Func = func;
            ExpressionList = expressionList;
        }

        protected override void PrintChildren(StringBuilder buf, int depth)
        {
            Func.Print(buf, depth);
            foreach (var child in ExpressionList)
            {
                child.Print(buf, depth);
            }
        }
    }

    class AstVariable : AstExpression
    {
        public string Name { get; private set; }

        public AstVariable(Token from, string name)
            : base(from, NodeType.VARIABLE)
        {
            Name = name;
        }

        public override string ToString()
        {
            return base.ToString() + " " + Name;
        }
    }

    class AstConstant : AstExpression
    {
        public Value Value { get; private set; }

        public AstConstant(Token from, Value value)
            : base(from, NodeType.CONSTANT)
        {
            Value = value;
        }

        public override string ToString()
        {
            return base.ToString() + " " + Value.Type + " " + Value.ToString();
        }
    }
}
