using System;

namespace DollsLang
{
    public class LangException : Exception
    {
        public LangException() : this("Language error") { }

        public LangException(string message)
            : base(message)
        { }

        public LangException(string message, Exception inner)
            : base(message, inner)
        { }
    }

    public class FatalLangException : LangException
    {
        public FatalLangException() : this("Language system fatal error") { }

        public FatalLangException(string message)
            : base(message)
        { }

        public FatalLangException(string message, Exception inner)
            : base(message, inner)
        { }
    }

    public class LexicalLangException : LangException
    {
        public LexicalLangException() : this("Lexical error") { }

        public LexicalLangException(string message)
            : base(message)
        { }

        public LexicalLangException(string message, Exception inner)
            : base(message, inner)
        { }
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
    }

    public class RuntimeLangException : LangException
    {
        public int Line { get; set; } = -1;
        public int Column { get; set; } = -1;

        public RuntimeLangException() : this("Runtime error") { }

        public RuntimeLangException(string message)
            : base(message)
        { }

        public RuntimeLangException(string message, Exception inner)
            : base(message, inner)
        { }

        public override string Message => string.Format(
            "Runtime Error at line {0}, column {1} {2}",
            Line, Column, base.Message);
    }
}
