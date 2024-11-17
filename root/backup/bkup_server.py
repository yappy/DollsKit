#! /usr/bin/env python3

import argparse
import datetime
import pathlib
import subprocess
import shutil
import re

INBOX_README = """
This is backup inbox.

file name format is:
  name_????????_??????.ext.ext2...
  yyyymmdd: date
  hhmmss  : time

AFTER your transfer is completed, create md5 file:
  <your file name>.md5sum
"""

def exec(cmd: list[str], fout=None):
	print(f"EXEC: {' '.join(cmd)}")
	if fout is not None:
		print("(stdout is redirected)")
	subprocess.run(cmd, check=True, stdout=fout)

def mount_check(dst: pathlib.Path):
	print("Destination mount check...")
	exec(["mountpoint", str(dst)])
	print()

# Scan inbox/*
def process_inbox(inbox_dir: pathlib.Path, repo_dir: pathlib.Path):
	print("process_inbox")
	print(f"inbox_dir: {inbox_dir}")
	print(f"repo_dir : {repo_dir}")
	dt_now = datetime.datetime.now()
	date_now = dt_now.strftime("%Y%m%d")
	time_now = dt_now.strftime("%H%M%S")

	processed = 0
	for p in inbox_dir.iterdir():
		if not p.exists():
			continue
		if not p.is_file() or p.is_symlink():
			print("Warning: {} is not a regular file")
			continue

		name = p.name
		complete_name = name + ".complete"
		complete_path = inbox_dir / complete_name
		print(f"process: {name}")

		if complete_path.exists():
			print(f"  {complete_name} found")
			match = re.fullmatch(r'(.*)[\-\_](\d{8})[\-\_](\d{6})\.(.*)', name)
			if match is None:
				m2 = re.fullmatch(r'([^\.]*)\.(.*)', name)
				if m2 is not None:
					name = f"{m2.group(1)}_{date_now}_{time_now}.{m2.group(2)}"
				else:
					print("invalid file name")
					continue
			move_to = repo_dir / name
			print(f"  move to {move_to}")
			p.rename(move_to)
			complete_path.unlink(True)

			processed += 1
		else:
			print(f"  {complete_name} not found")

	print("process_inbox OK")
	print(f"  processed: {processed}")
	print()

def main():
	parser = argparse.ArgumentParser(description="Auto backup script")
	parser.add_argument("dir", help="backup root dir")
	parser.add_argument("--mount-check", action="store", help="check if the specified path is a mountpoint")
	args = parser.parse_args()

	dir = pathlib.Path(args.dir).resolve()

	# mountpoint check
	if args.mount_check is not None:
		mount_check(args.mount_check)

	total, used, free = map(lambda s: s // (1024 ** 3), shutil.disk_usage(dir))
	print(f"DIR: {dir}")
	print(f"{used} used, {free} free / {total} total GiB")

	inbox_dir = dir / "inbox"
	repo_dir = dir / "repo"
	inbox_dir.mkdir(parents=True, exist_ok=True)
	repo_dir.mkdir(parents=True, exist_ok=True)
	print(f"INBOX: {inbox_dir}")
	print(f"PEPO : {repo_dir}")
	print()

	process_inbox(inbox_dir, repo_dir)

	print("OK!")

if __name__ == '__main__':
	main()
