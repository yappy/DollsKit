using System;
using System.Text;
using DollsLang;

namespace LangTest
{
    class Program
    {
        static void Main(string[] args)
        {
            var lexer = new Lexer();

            string test1 = "t = 3 s = 3.14 u = 1e10\n";
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
