import argparse
import datetime
import pathlib
import subprocess
import tempfile
import shutil
import glob

def exec(cmd):
	print(f"EXEC: {cmd}")
	subprocess.run(cmd, check=True)

def mount_check(dst):
	print("Destination mount check...")
	exec(["mountpoint", str(dst)])
	print()

def remove_old_files(dst, reserved_size):
	files = glob.glob(str(dst) + "/*.tar.bz2")
	files.sort()
	files = list(map(pathlib.Path, files))
	print(f"Old files: {files}")

	reserved_size /= (1024 ** 3)
	print(f"Allocate free area: {reserved_size} GiB")
	while files:
		total, used, free = map(lambda s: s / (1024 ** 3), shutil.disk_usage(dst))
		print(f"total: {total}, used: {used}, free: {free}")
		if free >= reserved_size:
			break
		file = files.pop(0)
		print(f"Delete {file}")
		file.unlink()
	print("Deleting old files completed")

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

	with tempfile.NamedTemporaryFile() as tf:
		print(f"Temp file created: {tf.name}")
		# -a: Use archive suffix to determine the compression program.
		# -c: Create new.
		# -f: Specify file name.
		cmd = ["tar", "-C", str(rsync_dst), "-acvf", tf.name, "."]
		exec(cmd)

		print(f"Copy {tf.name} -> {ar_dst}")
		shutil.copyfile(tf.name, str(ar_dst))
		print(f"Delete temp file: {tf.name}")
		# close and delete
	print()

def main():
	parser = argparse.ArgumentParser(description="Auto backup script")
	parser.add_argument("src", help="backup source root")
	parser.add_argument("dst", help="backup destination root")
	parser.add_argument("--mount-check", action="store_true", help="check if dst is a mountpoint")
	parser.add_argument("--reserved-size", type=int, default=10, help="delete old files to allocate free area (GiB) (default=10)")
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
	remove_old_files(dst, args.reserved_size << 30)
	rsync(src, rsync_dst, ex_list, args.dry_run)
	archive(rsync_dst, ar_dst, args.dry_run)

	print("OK!")

if __name__ == '__main__':
	main()
