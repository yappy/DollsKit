using System.Collections.Generic;
using System.Text.RegularExpressions;

namespace DollsLang
{
    public class Lexer
    {
        private struct Target
        {
            public Regex regex;
            public TokenType type;
        }

        private readonly char lineComment = '#';
        private readonly Regex skipRegex = new Regex(@"\G\s*");
        private readonly List<Target> targetList = new List<Target>();

        public Lexer()
        {
            targetList.Add(new Target { regex = new Regex(@"\G\+"), type = TokenType.PLUS });
            targetList.Add(new Target { regex = new Regex(@"\G\-"), type = TokenType.MINUS });
            targetList.Add(new Target { regex = new Regex(@"\G\*"), type = TokenType.MUL });
            targetList.Add(new Target { regex = new Regex(@"\G\/"), type = TokenType.DIV });
            targetList.Add(new Target { regex = new Regex(@"\G\%"), type = TokenType.MOD });

            targetList.Add(new Target { regex = new Regex(@"\G<="), type = TokenType.LE });
            targetList.Add(new Target { regex = new Regex(@"\G>="), type = TokenType.GE });
            targetList.Add(new Target { regex = new Regex(@"\G<"), type = TokenType.LT });
            targetList.Add(new Target { regex = new Regex(@"\G>"), type = TokenType.GT });
            targetList.Add(new Target { regex = new Regex(@"\G=="), type = TokenType.EQ });
            targetList.Add(new Target { regex = new Regex(@"\G!="), type = TokenType.NE });

            targetList.Add(new Target { regex = new Regex(@"\G\&\&"), type = TokenType.AND });
            targetList.Add(new Target { regex = new Regex(@"\G\|\|"), type = TokenType.OR });
            targetList.Add(new Target { regex = new Regex(@"\G\!"), type = TokenType.NOT });

            targetList.Add(new Target { regex = new Regex(@"\G\="), type = TokenType.ASSIGN });
            targetList.Add(new Target { regex = new Regex(@"\G\("), type = TokenType.LPAREN });
            targetList.Add(new Target { regex = new Regex(@"\G\)"), type = TokenType.RPAREN });
            targetList.Add(new Target { regex = new Regex(@"\G\{"), type = TokenType.LBRACE });
            targetList.Add(new Target { regex = new Regex(@"\G\}"), type = TokenType.RBRACE });
            targetList.Add(new Target { regex = new Regex(@"\G\,"), type = TokenType.COMMA });

            string idPat = @"\G[_a-zA-Z][_a-zA-Z0-9]*";
            targetList.Add(new Target { regex = new Regex(idPat), type = TokenType.ID });

            string floatPat = @"\G[0-9]+\.[0-9]+";
            targetList.Add(new Target { regex = new Regex(floatPat), type = TokenType.FLOAT });
            string floatExpPat = @"\G[0-9]+(?:\.[0-9]+)?[eE][\+\-]?[0-9]+";
            targetList.Add(new Target { regex = new Regex(floatExpPat), type = TokenType.FLOAT });

            string intPat = @"\G[0-9]+";
            targetList.Add(new Target { regex = new Regex(intPat), type = TokenType.INT });

            // "string"
            string strPat = @"\G""(?:\\""|[^""])*""";
            targetList.Add(new Target { regex = new Regex(strPat), type = TokenType.STRING });
        }

        public List<Token> Process(string source)
        {
            var result = new List<Token>();

            string[] lines = source.Split('\n');
            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i];
                int lineNo = i + 1;

                int x = 0;
                while (true)
                {
                    // skip space
                    Match skip = skipRegex.Match(line, x);
                    x += skip.Length;
                    // end of line
                    if (x >= line.Length)
                    {
                        break;
                    }
                    // line comment
                    if (line[x] == lineComment)
                    {
                        break;
                    }

                    int columnNo = x + 1;
                    bool find = false;
                    foreach (var target in targetList)
                    {
                        Match match = target.regex.Match(line, x);
                        if (match.Success)
                        {
                            var token = new Token(CheckReserved(target.type, match.Value),
                                match.Value, lineNo, columnNo);
                            result.Add(token);

                            find = true;
                            x += match.Length;
                            break;
                        }
                    }
                    if (!find)
                    {
                        throw new LexicalLangException(
                            string.Format("Lexical Error at line {0}, column {1}", lineNo, columnNo));
                    }
                }
            }
            // EOF token
            result.Add(new Token(TokenType.EOF, "", lines.Length, 0));

            return result;
        }

        private TokenType CheckReserved(TokenType orgType, string text)
        {
            // detected as ID
            if (orgType != TokenType.ID)
                return orgType;

            switch (text)
            {
                case "nil":
                    return TokenType.NIL;
                case "false":
                    return TokenType.FALSE;
                case "true":
                    return TokenType.TRUE;
                case "if":
                    return TokenType.IF;
                case "elif":
                    return TokenType.ELIF;
                case "else":
                    return TokenType.ELSE;
                case "while":
                    return TokenType.WHILE;
                case "for":
                    return TokenType.FOR;
                default:
                    return orgType;
            }
        }
    }
}
