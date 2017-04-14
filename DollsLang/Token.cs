namespace DollsLang
{
    public enum TokenType
    {
        EOF,
        IF, ELIF, ELSE, WHILE,
        ASSIGN,
        LPAREN, RPAREN, LBRACE, RBRACE, BAR,
        PLUS, MINUS, MUL, DIV, MOD,
        LT, LE, GT, GE, EQ, NE,
        AND, OR, NOT,
        NIL, FALSE, TRUE,
        ID, STRING, INT, FLOAT,
    }

    public class Token
    {
        public TokenType Type { get; private set; }
        public string Text { get; private set; }
        public int Line { get; private set; }
        public int Column { get; private set; }

        public Token(TokenType type, string text, int line, int column)
        {
            Type = type;
            Text = text;
            Line = line;
            Column = column;
        }

        public override string ToString()
        {
            return string.Format("[({0}:{1}){2}: {3}]", Line, Column, Type, Text);
        }
    }
}
