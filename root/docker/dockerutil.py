#!/usr/bin/env python3

import sys
import subcmd


def usage(argv0: str):
    print(f"{argv0} SUBCMD [args...]")
    print("Subcommands")
    for _func, name, desc in subcmd.command_table:
        print(f"* {name}")
        print(f"    {desc}")


def main(argv: list[str]):
    if len(argv) < 2 or argv[1] == "--help" or argv[1] == "-h":
        usage(argv[0])
        sys.exit(0)

    cmd = argv[1]
    args = argv[2:]
    found = False
    for func, name, _desc in subcmd.command_table:
        if name == cmd:
            func([cmd] + args)
            found = True
            break
    if not found:
        print(f"Subcommand not found: {cmd}")
        print()
        usage(argv[0])
        sys.exit(1)


if __name__ == '__main__':
    main(sys.argv)
