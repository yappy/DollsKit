using System;
using System.Drawing;
using System.Text;
using DollsLang;
using System.Threading;
using System.Threading.Tasks;

namespace LangTest
{
    class Program
    {
        static void Main(string[] args)
        {
            while (true)
            {
                string src = Console.ReadLine();
                if (src.Length == 0)
                {
                    break;
                }
                Console.WriteLine(interpret(src));
            }
        }

        static String interpret(String src)
        {
            try
            {
                var lexer = new Lexer();
                var tokenList = lexer.Process(src);
                var parser = new Parser();
                var program = parser.Parse(tokenList);
                var cancelSource = new CancellationTokenSource();
                var runtime = new Runtime(cancelSource.Token);
                runtime.LoadDefaultLibrary();
                string result;
                Bitmap graphicsResult;
                runtime.Execute(program, out result, out graphicsResult);
                if (graphicsResult != null)
                {
                    graphicsResult.Save("out.png");
                }
                return result;
            }
            catch (LangException e)
            {
                return e.Message;
            }
        }
    }
}
