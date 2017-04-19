using System;
using System.Collections.Generic;
using System.Text;

namespace DollsLang
{
    public class Parser
    {
        private List<Token> tokenList;
        private int readPtr;

        public Parser() { }

        public AstProgram Parse(List<Token> tokenList)
        {
            this.tokenList = tokenList;
            this.readPtr = 0;

            return Program();
        }

        /*
         * Program ::= (Statement)* <EOF>
         */
        private AstProgram Program()
        {
            var statements = new List<AstStatement>(); ;
            while (Peek() != TokenType.EOF)
            {
                statements.Add(Statement());
            }

            return new AstProgram(null, statements);
        }

        /*
         * Statement ::= IfElifElse
         * Statement ::= While
         * Statement ::= Expression
         */
        private AstStatement Statement()
        {
            switch (Peek(0))
            {
                case TokenType.IF:
                    return IfElifElse();
                case TokenType.WHILE:
                    return WhileLoop();
                default:
                    return Expression();
            }
        }

        /*
         * Block ::= <LBRACE> Statement* <RBRACE>
         */
        private List<AstStatement> Block()
        {
            var statList = new List<AstStatement>();

            Next(TokenType.LBRACE);
            while (Peek() != TokenType.RBRACE)
            {
                statList.Add(Statement());
            }
            Next(TokenType.RBRACE);

            return statList;
        }

        /*
         * IfElifElse ::= If Elif* Else?
         * If ::= <IF> <LPAREN> Expression <RPAREN> Block
         * Elif ::= <ELIF> <LPAREN> Expression <RPAREN> Block
         * Else ::= <ELSE> Block
         */
        private AstIf IfElifElse()
        {
            var condBodyList = new List<AstIf.CondAndBody>();

            // If
            Token ifToken = Next(TokenType.IF);
            Next(TokenType.LPAREN);
            AstExpression ifCond = Expression();
            Next(TokenType.RPAREN);
            var ifStatList = Block();
            condBodyList.Add(new AstIf.CondAndBody(ifCond, ifStatList));

            // Elif*
            while (Peek() == TokenType.ELIF)
            {
                Next(TokenType.ELIF);
                Next(TokenType.LPAREN);
                AstExpression elifCond = Expression();
                Next(TokenType.RPAREN);
                var elifStatList = Block();
                condBodyList.Add(new AstIf.CondAndBody(elifCond, elifStatList));
            }

            // Else?
            if (Peek() == TokenType.ELSE)
            {
                Next(TokenType.ELSE);
                var elseStatList = Block();
                condBodyList.Add(new AstIf.CondAndBody(null, elseStatList));
            }

            return new AstIf(ifToken, condBodyList);
        }

        /*
         * While ::= <WHILE> <LPAREN> Expression <RPAREN> <LBRACE> Statement* <RBRACE>
         */
        private AstWhile WhileLoop()
        {
            Token whileToken = Next(TokenType.WHILE);
            Next(TokenType.LPAREN);
            AstExpression cond = Expression();
            Next(TokenType.RPAREN);

            var statList = new List<AstStatement>();
            Next(TokenType.LBRACE);
            while (Peek() != TokenType.RBRACE)
            {
                statList.Add(Statement());
            }
            Next(TokenType.RBRACE);

            return new AstWhile(whileToken, cond, statList);
        }

        /*
         * Expression ::= Assign
         */
        private AstExpression Expression()
        {
            return Assign();
        }

        /*
         * Assign ::= Or <ASSIGN> Assign
         * Assign ::= Or
         */
        private AstExpression Assign()
        {
            var left = Or();
            if (Peek() == TokenType.ASSIGN)
            {
                Token assignToken = Next(TokenType.ASSIGN);
                if (left.Type == NodeType.Variable)
                {
                    // <ID> = Expression
                    var right = Assign();
                    string varName = ((AstVariable)left).Name;
                    return new AstAssign(assignToken, varName, right);
                }
                else if (left.Type == NodeType.ReadArray)
                {
                    // Expression [Expression] = Expression
                    var right = Assign();
                    var ltmp = (AstReadArray)left;
                    return new AstAssignArray(assignToken, ltmp.Array, ltmp.Index, right);
                }
                else
                {
                    throw CreateSyntaxError("Invalid assign");
                }
            }
            return left;
        }

        /*
         * Or ::= And (<OR> And)*
         */
        private AstExpression Or()
        {
            var left = And();
            while (Peek() == TokenType.OR)
            {
                Token orToken = Next(TokenType.OR);
                var right = And();
                left = new AstOperation(orToken, OperationType.Or, left, right);
            }
            return left;
        }

        /*
         * And ::= Equal (<AND> Equal)*
         */
        private AstExpression And()
        {
            var left = Equal();
            while (Peek() == TokenType.AND)
            {
                Token andToken = Next(TokenType.AND);
                var right = Equal();
                left = new AstOperation(andToken, OperationType.And, left, right);
            }
            return left;
        }

        /*
         * Equal ::= Compare ((<EQ> | <NE>) Compare)*
         */
        private AstExpression Equal()
        {
            var left = Compare();
            while (Peek() == TokenType.EQ || Peek() == TokenType.NE)
            {
                Token op = NextAny();
                var right = Compare();
                switch (op.Type)
                {
                    case TokenType.EQ:
                        left = new AstOperation(op, OperationType.EQ, left, right);
                        break;
                    case TokenType.NE:
                        left = new AstOperation(op, OperationType.NE, left, right);
                        break;
                    default:
                        throw new RuntimeLangException();
                }
            }
            return left;
        }

        /*
         * Compare ::= AddSub ((<LT> | <LE> | <GT> | <GE>) AddSub)*
         */
        private AstExpression Compare()
        {
            var left = AddSub();
            while (Peek() == TokenType.LT || Peek() == TokenType.LE ||
                Peek() == TokenType.GT || Peek() == TokenType.GE)
            {
                Token op = NextAny();
                var right = AddSub();
                switch (op.Type)
                {
                    case TokenType.LT:
                        left = new AstOperation(op, OperationType.LT, left, right);
                        break;
                    case TokenType.LE:
                        left = new AstOperation(op, OperationType.LE, left, right);
                        break;
                    case TokenType.GT:
                        left = new AstOperation(op, OperationType.GT, left, right);
                        break;
                    case TokenType.GE:
                        left = new AstOperation(op, OperationType.GE, left, right);
                        break;
                    default:
                        throw new RuntimeLangException();
                }
            }
            return left;
        }

        /*
         * AddSub ::= MulDiv ((<PLUS> | <MINUS>) MulDiv)*
         */
        private AstExpression AddSub()
        {
            var left = MulDiv();
            while (Peek() == TokenType.PLUS || Peek() == TokenType.MINUS)
            {
                Token op = NextAny();
                var right = MulDiv();
                switch (op.Type)
                {
                    case TokenType.PLUS:
                        left = new AstOperation(op, OperationType.Add, left, right);
                        break;
                    case TokenType.MINUS:
                        left = new AstOperation(op, OperationType.Sub, left, right);
                        break;
                    default:
                        throw new RuntimeLangException();
                }
            }
            return left;
        }

        /*
         * MulDiv ::= Unary ((<MUL> | <DIV> | <REM>) Unary)*
         */
        private AstExpression MulDiv()
        {
            var left = Unary();
            while (Peek() == TokenType.MUL || Peek() == TokenType.DIV ||
                Peek() == TokenType.MOD)
            {
                Token op = NextAny();
                var right = Unary();
                switch (op.Type)
                {
                    case TokenType.MUL:
                        left = new AstOperation(op, OperationType.Mul, left, right);
                        break;
                    case TokenType.DIV:
                        left = new AstOperation(op, OperationType.Div, left, right);
                        break;
                    case TokenType.MOD:
                        left = new AstOperation(op, OperationType.Mod, left, right);
                        break;
                    default:
                        throw new RuntimeLangException();
                }
            }
            return left;
        }

        /*
         * Unary ::= (<PLUS> | <MINUS>) Unary
         * Unary ::= Postfixed
         */
        private AstExpression Unary()
        {
            switch (Peek())
            {
                case TokenType.PLUS:
                    Next(TokenType.PLUS);
                    return Unary();
                case TokenType.MINUS:
                    Token minusToken = Next(TokenType.MINUS);
                    return new AstOperation(minusToken, OperationType.Negative, Unary());
                default:
                    return Postfixed();
            }
        }

        /*
         * [Call function]
         * Postfixed ::= Value (<LPAREN> ExpressionListOrEmpty <RPAREN>)*
         * ExpressionListOrEmpty ::= eps | ExpressionList
         * ExpressionList ::= Expression (<COMMA> Expression)*
         *
         * [Read array]
         * Postfixed ::= Value (<LBRACKET> Expression <RBRACKET>)*
         */
        private AstExpression Postfixed()
        {
            var value = Value();
            while (Peek() == TokenType.LPAREN || Peek() == TokenType.LBRACKET)
            {
                if (Peek() == TokenType.LPAREN)
                {
                    var exprList = new List<AstExpression>();
                    Token lparenToken = Next(TokenType.LPAREN);
                    if (Peek() != TokenType.RPAREN)
                    {
                        exprList.Add(Expression());
                        while (Peek() == TokenType.COMMA)
                        {
                            Next(TokenType.COMMA);
                            exprList.Add(Expression());
                        }
                    }
                    Next(TokenType.RPAREN);

                    value = new AstFunctionCall(lparenToken, value, exprList);
                }
                else if (Peek() == TokenType.LBRACKET)
                {
                    Token lbracketToken = Next(TokenType.LBRACKET);
                    var index = Expression();
                    Next(TokenType.RBRACKET);

                    value = new AstReadArray(lbracketToken, value, index);
                }
                else
                {
                    throw new FatalLangException();
                }
            }
            return value;
        }

            /*
             * Value ::= <LPAREN> Expression <RPAREN>
             * Value ::= Array
             * Value ::= Function
             * Value ::= <NIL> | <FALSE> | <TRUE>
             * Value ::= <ID> | <STRING> | <INT> | <FLOAT>
             */
            private AstExpression Value()
        {
            AstExpression result;
            Token token;
            switch (Peek())
            {
                case TokenType.LPAREN:
                    Next(TokenType.LPAREN);
                    result = Expression();
                    Next(TokenType.RPAREN);
                    break;
                case TokenType.LBRACKET:
                    result = Array();
                    break;
                case TokenType.BAR:
                    result = Function();
                    break;
                case TokenType.NIL:
                    token = Next(TokenType.NIL);
                    result = new AstConstant(token, NilValue.Nil);
                    break;
                case TokenType.FALSE:
                    token = Next(TokenType.FALSE);
                    result = new AstConstant(token, BoolValue.False);
                    break;
                case TokenType.TRUE:
                    token = Next(TokenType.TRUE);
                    result = new AstConstant(token, BoolValue.True);
                    break;
                case TokenType.ID:
                    token = Next(TokenType.ID);
                    result = new AstVariable(token, token.Text);
                    break;
                case TokenType.STRING:
                    token = Next(TokenType.STRING);
                    result = new AstConstant(token,
                        new StringValue(NormalizeString(token.Text)));
                    break;
                case TokenType.INT:
                    token = Next(TokenType.INT);
                    result = new AstConstant(token,
                        new IntValue(ConvertToInt(token.Text)));
                    break;
                case TokenType.FLOAT:
                    token = Next(TokenType.FLOAT);
                    result = new AstConstant(token,
                        new FloatValue(ConvertToFloat(token.Text)));
                    break;
                default:
                    throw CreateSyntaxError();
            }
            return result;
        }

        /*
         * Array ::= <LBRACKET> ExpressionListOrEmpty <RBRACKET>
         * ExpressionListOrEmpty ::= eps | ExpressionList
         * ExpressionList ::= Expression (<COMMA> Expression)*
         */
        private AstExpression Array()
        {
            var exprList = new List<AstExpression>();

            Token startingToken = Next(TokenType.LBRACKET);
            if (Peek() != TokenType.RBRACKET)
            {
                exprList.Add(Expression());
                while (Peek() == TokenType.COMMA)
                {
                    Next(TokenType.COMMA);
                    exprList.Add(Expression());
                }
            }
            Next(TokenType.RBRACKET);

            return new AstConstructArray(startingToken, exprList);
        }

        /*
         * Function ::= <BAR> ParamListOrEmpty <BAR> Block
         * ParamListOrEmpty ::= eps | ParamList
         * ParamList ::= <ID> (<COMMA> <ID>)*
         */
        private AstConstant Function()
        {
            var paramList = new List<string>();
            Token startingToken = Next(TokenType.BAR);
            if (Peek() != TokenType.BAR)
            {
                paramList.Add(Next(TokenType.ID).Text);
                while (Peek() == TokenType.COMMA)
                {
                    Next(TokenType.COMMA);
                    paramList.Add(Next(TokenType.ID).Text);
                }
            }
            Next(TokenType.BAR);

            var body = Block();

            return new AstConstant(startingToken,
                new UserFunctionValue(paramList, body));
        }

        private TokenType Peek(int offset = 0)
        {
            int idx = Math.Min(readPtr + offset, tokenList.Count - 1);
            return tokenList[idx].Type;
        }

        private Token NextAny()
        {
            Token token = tokenList[readPtr];
            if (readPtr < tokenList.Count - 1)
            {
                readPtr++;
            }
            return token;
        }

        private Token Next(TokenType type)
        {
            if (Peek() != type)
            {
                throw CreateSyntaxError();
            }
            return NextAny();
        }

        private string NormalizeString(string src)
        {
            if (src.Length < 2 || src[0] != '"' || src[src.Length - 1] != '"')
            {
                throw new FatalLangException();
            }

            var result = new StringBuilder();
            for (int i = 1; i < src.Length - 1; i++)
            {
                if (src[i] == '\\')
                {
                    i++;
                    result.Append(src[i]);
                }
                else
                {
                    result.Append(src[i]);
                }
            }
            return result.ToString();
        }

        private int ConvertToInt(string src)
        {
            int result;
            if (int.TryParse(src, out result))
            {
                return result;
            }
            else
            {
                throw CreateSyntaxError("Convert failed: " + src);
            }
        }

        private double ConvertToFloat(string src)
        {
            double result;
            if (double.TryParse(src, out result))
            {
                return result;
            }
            else
            {
                throw CreateSyntaxError("Convert failed: " + src);
            }
        }

        private Exception CreateSyntaxError(string message = "")
        {
            Token token = tokenList[readPtr];
            return new SyntaxLangException(
                string.Format("Syntax Error at line {0}, column {1} {2}",
                token.Line, token.Column, message));
        }
    }
}
