using System;
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
            var lexer = new Lexer();

            string test1 = @"
a = 3
b = c = 3.14
x = 1.0 y = 2.0
e = 1e-15
s = ""this is \""string\""""

p(nil) p(false) p(true)
print(a, b, c)
print(x, y, e)
print(s)

while (y - x > e) {
  m = (y + x) / 2
  if (m * m > 2) { y = m }
  else { x = m }
}
p(m)

arr1 = []
arr2 = [1, 3.14, ""hello"", print]
print(arr1, arr2)
print(arr2[2])
arr1[2] = ""test""
print(arr1)
";

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

            var cancelSource = new CancellationTokenSource();
            var runtime = new Runtime(cancelSource.Token);
            runtime.LoadDefaultFunctions();
            cancelSource.CancelAfter(10 * 1000);
            try
            {
                string output = runtime.Execute(program);
                Console.WriteLine(output);
            }
            catch (OperationCanceledException)
            {
                Console.WriteLine("cancel!");
            }
            catch (LangException e)
            {
                Console.WriteLine(e.Message);
            }

            Console.Read();
        }
    }
}
