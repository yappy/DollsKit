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

            return program();
        }

        /*
         * Program ::= (Statement)* <EOF>
         */
        private AstProgram program()
        {
            var statements = new List<AstStatement>(); ;
            while (peek() != TokenType.EOF)
            {
                statements.Add(statement());
            }

            return new AstProgram(null, statements);
        }

        /*
         * Statement ::= IfElifElse
         * Statement ::= While
         * Statement ::= Expression
         */
        private AstStatement statement()
        {
            switch (peek(0))
            {
                case TokenType.IF:
                    return ifElifElse();
                case TokenType.WHILE:
                    return whileLoop();
                default:
                    return expression();
            }
        }

        /*
         * Block ::= <LBRACE> Statement* <RBRACE>
         */
        private List<AstStatement> block()
        {
            var statList = new List<AstStatement>();

            next(TokenType.LBRACE);
            while (peek() != TokenType.RBRACE)
            {
                statList.Add(statement());
            }
            next(TokenType.RBRACE);

            return statList;
        }

        /*
         * IfElifElse ::= If Elif* Else?
         * If ::= <IF> <LPAREN> Expression <RPAREN> Block
         * Elif ::= <ELIF> <LPAREN> Expression <RPAREN> Block
         * Else ::= <ELSE> Block
         */
        private AstIf ifElifElse()
        {
            var condBodyList = new List<AstIf.CondAndBody>();

            // If
            Token ifToken = next(TokenType.IF);
            next(TokenType.LPAREN);
            AstExpression ifCond = expression();
            next(TokenType.RPAREN);
            var ifStatList = block();
            condBodyList.Add(new AstIf.CondAndBody(ifCond, ifStatList));

            // Elif*
            while (peek() == TokenType.ELIF)
            {
                next(TokenType.ELIF);
                next(TokenType.LPAREN);
                AstExpression elifCond = expression();
                next(TokenType.RPAREN);
                var elifStatList = block();
                condBodyList.Add(new AstIf.CondAndBody(elifCond, elifStatList));
            }

            // Else?
            if (peek() == TokenType.ELSE)
            {
                next(TokenType.ELSE);
                var elseStatList = block();
                condBodyList.Add(new AstIf.CondAndBody(null, elseStatList));
            }

            return new AstIf(ifToken, condBodyList);
        }

        /*
         * While ::= <WHILE> <LPAREN> Expression <RPAREN> <LBRACE> Statement* <RBRACE>
         */
        private AstWhile whileLoop()
        {
            Token whileToken = next(TokenType.WHILE);
            next(TokenType.LPAREN);
            AstExpression cond = expression();
            next(TokenType.RPAREN);

            var statList = new List<AstStatement>();
            next(TokenType.LBRACE);
            while (peek() != TokenType.RBRACE)
            {
                statList.Add(statement());
            }
            next(TokenType.RBRACE);

            return new AstWhile(whileToken, cond, statList);
        }

        /*
         * Expression ::= Assign
         */
        private AstExpression expression()
        {
            return assign();
        }

        /*
         * Assign ::= Or <ASSIGN> Assign
         * Assign ::= Or
         */
        private AstExpression assign()
        {
            var left = or();
            if (peek() == TokenType.ASSIGN)
            {
                Token assignToken = next(TokenType.ASSIGN);
                if (left.Type == NodeType.Variable)
                {
                    // <ID> = Expression
                    var right = assign();
                    string varName = ((AstVariable)left).Name;
                    return new AstAssign(assignToken, varName, right);
                }
                else if (left.Type == NodeType.ReadArray)
                {
                    // Expression [Expression] = Expression
                    var right = assign();
                    var ltmp = (AstReadArray)left;
                    return new AstAssignArray(assignToken, ltmp.Array, ltmp.Index, right);
                }
                else
                {
                    throw createSyntaxError("Invalid assign");
                }
            }
            return left;
        }

        /*
         * Or ::= And (<OR> And)*
         */
        private AstExpression or()
        {
            var left = and();
            while (peek() == TokenType.OR)
            {
                Token orToken = next(TokenType.OR);
                var right = and();
                left = new AstOperation(orToken, OperationType.Or, left, right);
            }
            return left;
        }

        /*
         * And ::= Equal (<AND> Equal)*
         */
        private AstExpression and()
        {
            var left = equal();
            while (peek() == TokenType.AND)
            {
                Token andToken = next(TokenType.AND);
                var right = equal();
                left = new AstOperation(andToken, OperationType.And, left, right);
            }
            return left;
        }

        /*
         * Equal ::= Compare ((<EQ> | <NE>) Compare)*
         */
        private AstExpression equal()
        {
            var left = compare();
            while (peek() == TokenType.EQ || peek() == TokenType.NE)
            {
                Token op = nextAny();
                var right = compare();
                switch (op.Type)
                {
                    case TokenType.EQ:
                        left = new AstOperation(op, OperationType.EQ, left, right);
                        break;
                    case TokenType.NE:
                        left = new AstOperation(op, OperationType.NE, left, right);
                        break;
                    default:
                        throw new FatalLangException();
                }
            }
            return left;
        }

        /*
         * Compare ::= AddSub ((<LT> | <LE> | <GT> | <GE>) AddSub)*
         */
        private AstExpression compare()
        {
            var left = addSub();
            while (peek() == TokenType.LT || peek() == TokenType.LE ||
                peek() == TokenType.GT || peek() == TokenType.GE)
            {
                Token op = nextAny();
                var right = addSub();
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
                        throw new FatalLangException();
                }
            }
            return left;
        }

        /*
         * AddSub ::= MulDiv ((<PLUS> | <MINUS>) MulDiv)*
         */
        private AstExpression addSub()
        {
            var left = mulDiv();
            while (peek() == TokenType.PLUS || peek() == TokenType.MINUS)
            {
                Token op = nextAny();
                var right = mulDiv();
                switch (op.Type)
                {
                    case TokenType.PLUS:
                        left = new AstOperation(op, OperationType.Add, left, right);
                        break;
                    case TokenType.MINUS:
                        left = new AstOperation(op, OperationType.Sub, left, right);
                        break;
                    default:
                        throw new FatalLangException();
                }
            }
            return left;
        }

        /*
         * MulDiv ::= Unary ((<MUL> | <DIV> | <REM>) Unary)*
         */
        private AstExpression mulDiv()
        {
            var left = unary();
            while (peek() == TokenType.MUL || peek() == TokenType.DIV ||
                peek() == TokenType.MOD)
            {
                Token op = nextAny();
                var right = unary();
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
                        throw new FatalLangException();
                }
            }
            return left;
        }

        /*
         * Unary ::= (<PLUS> | <MINUS>) Unary
         * Unary ::= Postfixed
         */
        private AstExpression unary()
        {
            switch (peek())
            {
                case TokenType.PLUS:
                    next(TokenType.PLUS);
                    return unary();
                case TokenType.MINUS:
                    Token minusToken = next(TokenType.MINUS);
                    return new AstOperation(minusToken, OperationType.Negative, unary());
                default:
                    return postfixed();
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
        private AstExpression postfixed()
        {
            var value = factor();
            while (peek() == TokenType.LPAREN || peek() == TokenType.LBRACKET)
            {
                if (peek() == TokenType.LPAREN)
                {
                    var exprList = new List<AstExpression>();
                    Token lparenToken = next(TokenType.LPAREN);
                    if (peek() != TokenType.RPAREN)
                    {
                        exprList.Add(expression());
                        while (peek() == TokenType.COMMA)
                        {
                            next(TokenType.COMMA);
                            exprList.Add(expression());
                        }
                    }
                    next(TokenType.RPAREN);

                    value = new AstFunctionCall(lparenToken, value, exprList);
                }
                else if (peek() == TokenType.LBRACKET)
                {
                    Token lbracketToken = next(TokenType.LBRACKET);
                    var index = expression();
                    next(TokenType.RBRACKET);

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
         * Factor ::= <LPAREN> Expression <RPAREN>
         * Factor ::= Array
         * Factor ::= Function
         * Factor ::= <NIL> | <FALSE> | <TRUE>
         * Factor ::= <ID> | <STRING> | <INT> | <FLOAT>
         */
        private AstExpression factor()
        {
            AstExpression result;
            Token token;
            switch (peek())
            {
                case TokenType.LPAREN:
                    next(TokenType.LPAREN);
                    result = expression();
                    next(TokenType.RPAREN);
                    break;
                case TokenType.LBRACKET:
                    result = array();
                    break;
                case TokenType.BAR:
                    result = function();
                    break;
                case TokenType.NIL:
                    token = next(TokenType.NIL);
                    result = new AstConstant(token, NilValue.Nil);
                    break;
                case TokenType.FALSE:
                    token = next(TokenType.FALSE);
                    result = new AstConstant(token, BoolValue.False);
                    break;
                case TokenType.TRUE:
                    token = next(TokenType.TRUE);
                    result = new AstConstant(token, BoolValue.True);
                    break;
                case TokenType.ID:
                    token = next(TokenType.ID);
                    result = new AstVariable(token, token.Text);
                    break;
                case TokenType.STRING:
                    token = next(TokenType.STRING);
                    result = new AstConstant(token,
                        new StringValue(normalizeString(token.Text)));
                    break;
                case TokenType.INT:
                    token = next(TokenType.INT);
                    result = new AstConstant(token,
                        new IntValue(convertToInt(token.Text)));
                    break;
                case TokenType.FLOAT:
                    token = next(TokenType.FLOAT);
                    result = new AstConstant(token,
                        new FloatValue(convertToFloat(token.Text)));
                    break;
                default:
                    throw createSyntaxError("Invalid expression");
            }
            return result;
        }

        /*
         * Array ::= <LBRACKET> ExpressionListOrEmpty <RBRACKET>
         * ExpressionListOrEmpty ::= eps | ExpressionList
         * ExpressionList ::= Expression (<COMMA> Expression)*
         */
        private AstExpression array()
        {
            var exprList = new List<AstExpression>();

            Token startingToken = next(TokenType.LBRACKET);
            if (peek() != TokenType.RBRACKET)
            {
                exprList.Add(expression());
                while (peek() == TokenType.COMMA)
                {
                    next(TokenType.COMMA);
                    exprList.Add(expression());
                }
            }
            next(TokenType.RBRACKET);

            return new AstConstructArray(startingToken, exprList);
        }

        /*
         * Function ::= <BAR> ParamListOrEmpty <BAR> Block
         * ParamListOrEmpty ::= eps | ParamList
         * ParamList ::= <ID> (<COMMA> <ID>)*
         */
        private AstConstant function()
        {
            var paramList = new List<string>();
            Token startingToken = next(TokenType.BAR);
            if (peek() != TokenType.BAR)
            {
                paramList.Add(next(TokenType.ID).Text);
                while (peek() == TokenType.COMMA)
                {
                    next(TokenType.COMMA);
                    paramList.Add(next(TokenType.ID).Text);
                }
            }
            next(TokenType.BAR);

            var body = block();

            return new AstConstant(startingToken,
                new UserFunctionValue(paramList, body));
        }

        private TokenType peek(int offset = 0)
        {
            int idx = Math.Min(readPtr + offset, tokenList.Count - 1);
            return tokenList[idx].Type;
        }

        private Token nextAny()
        {
            Token token = tokenList[readPtr];
            if (readPtr < tokenList.Count - 1)
            {
                readPtr++;
            }
            return token;
        }

        private Token next(TokenType type)
        {
            if (peek() != type)
            {
                throw createSyntaxError($"<{type}> is required");
            }
            return nextAny();
        }

        private string normalizeString(string src)
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

        private int convertToInt(string src)
        {
            int result;
            if (int.TryParse(src, out result))
            {
                return result;
            }
            else
            {
                throw createSyntaxError($"Convert failed: {src}");
            }
        }

        private double convertToFloat(string src)
        {
            double result;
            if (double.TryParse(src, out result))
            {
                return result;
            }
            else
            {
                throw createSyntaxError($"Convert failed: {src}");
            }
        }

        private SyntaxLangException createSyntaxError(string message = "")
        {
            Token token = tokenList[readPtr];
            return new SyntaxLangException(message, token.Line, token.Column);
        }
    }
}
