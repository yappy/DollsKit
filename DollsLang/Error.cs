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

    public class LexicalLangException : Exception
    {
        public LexicalLangException() : this("Lexical error") { }

        public LexicalLangException(string message)
            : base(message)
        { }

        public LexicalLangException(string message, Exception inner)
            : base(message, inner)
        { }
    }

    public class SyntaxLangException : Exception
    {
        public SyntaxLangException() : this("Syntax error") { }

        public SyntaxLangException(string message)
            : base(message)
        { }

        public SyntaxLangException(string message, Exception inner)
            : base(message, inner)
        { }
    }

    public class RuntimeLangException : Exception
    {
        public RuntimeLangException() : this("Runtime error") { }

        public RuntimeLangException(string message)
            : base(message)
        { }

        public RuntimeLangException(string message, Exception inner)
            : base(message, inner)
        { }
    }
}
