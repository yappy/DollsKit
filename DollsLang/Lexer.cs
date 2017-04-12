using System;
using System.Collections.Generic;
using System.Text;
using System.Text.RegularExpressions;

namespace DollsLang
{
    class Lexer
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

            targetList.Add(new Target { regex = new Regex(@"\G\&"), type = TokenType.AND });
            targetList.Add(new Target { regex = new Regex(@"\G\|"), type = TokenType.OR });
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
                        throw new LexicalLangException(string.Format("Lexical Error at line {0}, column {1}", lineNo, columnNo));
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

    class Program
    {
        static void Main(string[] args)
        {
            var lexer = new Lexer();

            string test1 = "z = 1 / 0 t = 3 s = 3.14 u = 1e10\n";
            test1 += "x = 5 + t * -2 \n";
            test1 += "s = \"this is string\" \n";
            test1 += "p(nil) p(false) p(true)\n";
            test1 += "print(t x s) \n";
            test1 += "if(x>0){print(1)}elif(x<-10){print(2)}else{print(3)}\n";
            test1 += "i=5 while(i>0){print(i) i=i-1}\n";

            StringBuilder test2 = new StringBuilder("a=");
            const int depth = 100;
            for (int i = 0; i < depth; i++)
            {
                test2.Append('(');
            }
            test2.Append('1');
            for (int i = 0; i < depth; i++)
            {
                test2.Append(')');
            }

            var tokenList = lexer.Process(test1);
            foreach (var token in tokenList)
            {
                Console.WriteLine(token);
            }
            Console.WriteLine();

            var parser = new Parser();
            var program = parser.Parse(tokenList);
            var buf = new StringBuilder();
            program.Print(buf, 0);
            Console.WriteLine(buf);

            var runtime = new Runtime();
            runtime.LoadDefaultFunctions();
            runtime.Execute(program);

            tokenList = lexer.Process(test2.ToString());
            program = parser.Parse(tokenList);
            buf.Clear();
            program.Print(buf, 0);
            Console.WriteLine(buf);

            Console.Read();
        }
    }
}
