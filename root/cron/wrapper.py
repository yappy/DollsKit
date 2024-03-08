#! /usr/bin/env python3

import argparse
import subprocess

def main():
	parser = argparse.ArgumentParser(description="")
	parser.add_argument("--cmd", required=True, help="command line to be executed")
	parser.add_argument("--ok", help="command line on OK")
	parser.add_argument("--fail", help="command line on FAIL")
	args = parser.parse_args()

	try:
		print("EXEC:", args.cmd)
		subprocess.run(args.cmd, shell=True, check=True)
		if args.ok is not None:
			try:
				print("EXEC:", args.ok)
				subprocess.run(args.ok, shell=True, check=True)
			except subprocess.CalledProcessError as e:
				print(e)
	except subprocess.CalledProcessError as e:
		print(e)
		if args.fail is not None:
			try:
				print("EXEC:", args.fail)
				subprocess.run(args.fail, shell=True, check=True)
			except subprocess.CalledProcessError as e:
				print(e)

if __name__ == '__main__':
	main()
