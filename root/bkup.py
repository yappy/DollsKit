import argparse
import datetime
import pathlib
import subprocess
import shutil

def exec(cmd):
	print(f"EXEC: {cmd}")
	subprocess.run(cmd, check=True)

def mount_check(dst):
	print("Destination mount check...")
	exec(["mountpoint", str(dst)])
	print()

def rsync(src, rsync_dst, ex_list, dry_run):
	print ("rsync...")
	cmd = ["rsync", "-aur", "--stats", "--delete"]
	for ex in ex_list:
		cmd.append(f"--exclude-from={ex}")
	if dry_run:
		cmd.append("-n")
	cmd += [str(src), str(rsync_dst)]
	exec(cmd)
	print()

def archive(rsync_dst, ar_dst, dry_run):
	print("tar...")
	if dry_run:
		print("skip by dry-run")
		return
	# -a: Use archive suffix to determine the compression program.
	# -c: Create new.
	# -f: Specify file name.
	cmd = ["tar", "-C", str(rsync_dst), "-acvf", str(ar_dst), "."]
	exec(cmd)
	print()

def main():
	parser = argparse.ArgumentParser(description="Auto backup script")
	parser.add_argument("src", help="backup source root")
	parser.add_argument("dst", help="backup destination root")
	parser.add_argument("--mount-check", action="store_true", help="check if dst is a mountpoint")
	parser.add_argument("--exclude-from", action="append", help="check if dst is a mountpoint")
	parser.add_argument("-n", "--dry-run", action="store_true", help="rsync dry-run")
	args = parser.parse_args()

	dt_now = datetime.datetime.now()
	dt_str = dt_now.strftime('%Y%m%d_%H%M%S')

	src = pathlib.Path(args.src).resolve()
	dst = pathlib.Path(args.dst).resolve()
	rsync_dst = dst / "backup"
	ar_dst = dst / f"{dt_str}.tar.bz2"
	ex_list = list(map(lambda s: pathlib.Path(s).resolve(), args.exclude_from))
	print(f"Date: {dt_str}")
	print(f"SRC: {src}")
	print(f"DST: {dst}")
	print(f"RSYNC DST: {rsync_dst}")
	print(f"AR DST: {ar_dst}")
	print(f"Mount Check: {args.mount_check}")
	print(f"Exclude From: {list(map(str, ex_list))}")
	print(f"Dry Run: {args.dry_run}")
	print()

	if args.mount_check:
		mount_check(dst)
	rsync(src, rsync_dst, ex_list, args.dry_run)
	archive(rsync_dst, ar_dst, args.dry_run)

if __name__ == '__main__':
	main()
