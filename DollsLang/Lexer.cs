using System.Collections.Generic;
using System.Text.RegularExpressions;

namespace DollsLang
{
    public class Lexer
    {
        private struct Target
        {
            public Regex Regex;
            public TokenType Type;
        }

        private static readonly char LineComment = '#';
        private static readonly Regex SkipRegex = new Regex(@"\G\s*");
        private static readonly IList<Target> TargetList;

        static Lexer()
        {
            var targetList = new List<Target>();

            targetList.Add(new Target { Regex = new Regex(@"\G\+"), Type = TokenType.PLUS });
            targetList.Add(new Target { Regex = new Regex(@"\G\-"), Type = TokenType.MINUS });
            targetList.Add(new Target { Regex = new Regex(@"\G\*"), Type = TokenType.MUL });
            targetList.Add(new Target { Regex = new Regex(@"\G\/"), Type = TokenType.DIV });
            targetList.Add(new Target { Regex = new Regex(@"\G\%"), Type = TokenType.MOD });

            targetList.Add(new Target { Regex = new Regex(@"\G<="), Type = TokenType.LE });
            targetList.Add(new Target { Regex = new Regex(@"\G>="), Type = TokenType.GE });
            targetList.Add(new Target { Regex = new Regex(@"\G<"), Type = TokenType.LT });
            targetList.Add(new Target { Regex = new Regex(@"\G>"), Type = TokenType.GT });
            targetList.Add(new Target { Regex = new Regex(@"\G=="), Type = TokenType.EQ });
            targetList.Add(new Target { Regex = new Regex(@"\G!="), Type = TokenType.NE });

            targetList.Add(new Target { Regex = new Regex(@"\G\&\&"), Type = TokenType.AND });
            targetList.Add(new Target { Regex = new Regex(@"\G\|\|"), Type = TokenType.OR });
            targetList.Add(new Target { Regex = new Regex(@"\G\!"), Type = TokenType.NOT });

            targetList.Add(new Target { Regex = new Regex(@"\G\="), Type = TokenType.ASSIGN });
            targetList.Add(new Target { Regex = new Regex(@"\G\("), Type = TokenType.LPAREN });
            targetList.Add(new Target { Regex = new Regex(@"\G\)"), Type = TokenType.RPAREN });
            targetList.Add(new Target { Regex = new Regex(@"\G\{"), Type = TokenType.LBRACE });
            targetList.Add(new Target { Regex = new Regex(@"\G\}"), Type = TokenType.RBRACE });
            targetList.Add(new Target { Regex = new Regex(@"\G\["), Type = TokenType.LBRACKET });
            targetList.Add(new Target { Regex = new Regex(@"\G\]"), Type = TokenType.RBRACKET });
            targetList.Add(new Target { Regex = new Regex(@"\G\|"), Type = TokenType.BAR });
            targetList.Add(new Target { Regex = new Regex(@"\G\,"), Type = TokenType.COMMA });

            string idPat = @"\G[_a-zA-Z][_a-zA-Z0-9]*";
            targetList.Add(new Target { Regex = new Regex(idPat), Type = TokenType.ID });

            string floatPat = @"\G[0-9]+\.[0-9]+";
            targetList.Add(new Target { Regex = new Regex(floatPat), Type = TokenType.FLOAT });
            string floatExpPat = @"\G[0-9]+(?:\.[0-9]+)?[eE][\+\-]?[0-9]+";
            targetList.Add(new Target { Regex = new Regex(floatExpPat), Type = TokenType.FLOAT });

            string intPat = @"\G[0-9]+";
            targetList.Add(new Target { Regex = new Regex(intPat), Type = TokenType.INT });

            // "string"
            string strPat = @"\G""(?:\\""|[^""])*""";
            targetList.Add(new Target { Regex = new Regex(strPat), Type = TokenType.STRING });

            TargetList = targetList.AsReadOnly();
        }

        public Lexer()
        { }

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
                    Match skip = SkipRegex.Match(line, x);
                    x += skip.Length;
                    // end of line
                    if (x >= line.Length)
                    {
                        break;
                    }
                    // line comment
                    if (line[x] == LineComment)
                    {
                        break;
                    }

                    int columnNo = x + 1;
                    bool find = false;
                    foreach (var target in TargetList)
                    {
                        Match match = target.Regex.Match(line, x);
                        if (match.Success)
                        {
                            var token = new Token(checkReserved(target.Type, match.Value),
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
                            $"'{line[x]}'", lineNo, columnNo);
                    }
                }
            }
            // EOF token
            result.Add(new Token(TokenType.EOF, "", lines.Length, 0));

            return result;
        }

        private TokenType checkReserved(TokenType orgType, string text)
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
                default:
                    return orgType;
            }
        }
    }
}
