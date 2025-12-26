#!/usr/bin/env python3

import argparse
import sys
import subprocess


def exec(cmdline: list[str]):
    print(f"EXEC: {' '.join(cmdline)}")
    subprocess.run(cmdline, check=True)

def main(argv: list[str]):
    parser = argparse.ArgumentParser(
        prog=argv[0],
        description="Launch sh with busybox image",
        epilog='If you want to execute multiple commands, `sh -c "cmd1 && comd2 && ..."`',
    )
    parser.add_argument("CMD", nargs="*",
                        help="Command line (if not specified, launch interactive shell)")

    args = parser.parse_args(argv[1:])

    cmd = ["docker", "run", "--rm", "-it", "busybox"]
    if args.CMD:
        cmd.extend(args.CMD)
    exec(cmd)

if __name__ == '__main__':
    main(sys.argv)
