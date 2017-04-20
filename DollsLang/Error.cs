using System;

namespace DollsLang
{
    public class FatalLangException : Exception
    {
        public FatalLangException() : this("Language system fatal error") { }

        public FatalLangException(string message)
            : base(message)
        { }

        public FatalLangException(string message, Exception inner)
            : base(message, inner)
        { }
    }

    public class LangException : Exception
    {
        public int Line { get; set; } = -1;
        public int Column { get; set; } = -1;

        public LangException() : this("Language error") { }

        public LangException(string message)
            : base(message)
        { }

        public LangException(string message, Exception inner)
            : base(message, inner)
        { }
    }

    public class LexicalLangException : LangException
    {
        public LexicalLangException() : this("Lexical error") { }

        public LexicalLangException(string message)
            : base(message)
        { }

        public LexicalLangException(string message, int line, int column)
            : base(message)
        {
            Line = line;
            Column = column;
        }

        public override string Message =>
            $"Lexical Error at line {Line}, column {Column}: {base.Message}";
    }

    public class SyntaxLangException : LangException
    {
        public SyntaxLangException() : this("Syntax error") { }

        public SyntaxLangException(string message)
            : base(message)
        { }

        public SyntaxLangException(string message, Exception inner)
            : base(message, inner)
        { }

        public SyntaxLangException(string message, int line, int column)
            : base(message)
        {
            Line = line;
            Column = column;
        }

        public override string Message =>
            $"Syntax Error at line {Line}, column {Column}: {base.Message}";
    }

    public class RuntimeLangException : LangException
    {
        public RuntimeLangException() : this("Runtime error") { }

        public RuntimeLangException(string message)
            : base(message)
        { }

        public RuntimeLangException(string message, Exception inner)
            : base(message, inner)
        { }

        public override string Message =>
            $"Runtime Error at line {Line}, column {Column}: {base.Message}";
    }
}
