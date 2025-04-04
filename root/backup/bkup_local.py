#! /usr/bin/env python3

import argparse
import datetime
import getpass
import platform
import pathlib
import subprocess
import shutil
import glob
import os

RSYNC_DIR_NAME = "backup"
LATEST_SLINK_NAME = "latest"
ARCHIVE_EXT = "tar.bz2"

def exec(cmd: list[str], fout=None):
	print(f"EXEC: {' '.join(cmd)}")
	if fout is not None:
		print("(stdout is redirected)")
	subprocess.run(cmd, check=True, stdout=fout)

def mount_check(dst: pathlib.Path):
	print("Destination mount check...")
	exec(["mountpoint", str(dst)])
	print()

def delete_old_files(dst: pathlib.Path, keep_count: int):
	files = glob.glob(str(dst) + f"/*.{ARCHIVE_EXT}")
	files.sort()
	files = list(map(pathlib.Path, files))

	print(f"Delete old files: keep={keep_count}, files={files}")
	while len(files) > keep_count:
		file = files.pop(0)
		print(f"Delete {file}")
		file.unlink()
	print("Deleting old files completed")

def allocate_size(dst: pathlib.Path, reserved_size: int):
	files = glob.glob(str(dst) + f"/*.{ARCHIVE_EXT}")
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
	print("Allocating free area completed")

def dump_db(rsync_dst: pathlib.Path, dump_command: str, db: str, dry_run: bool):
	print("DB dump...")
	if dry_run:
		print("skip by dry-run")
		return

	dst_sql = rsync_dst / "db.sql"
	with dst_sql.open(mode="wb") as fout:
		os.fchmod(fout.fileno(), 0o600)
		cmd = [dump_command, "--databases", db]
		exec(cmd, fout)
	print()

def rsync(src: pathlib.Path, rsync_dst: pathlib.Path, ex_list: list[str], dry_run: bool):
	print ("rsync...")
	cmd = ["rsync", "-aur", "--stats", "--delete"]
	for ex in ex_list:
		cmd.append(f"--exclude-from={ex}")
	if dry_run:
		cmd.append("-n")
	cmd += [str(src), str(rsync_dst)]
	exec(cmd)
	print()

def archive(rsync_dst: pathlib.Path, ar_dst: pathlib.Path, dry_run: bool):
	print("tar...")
	if dry_run:
		print("skip by dry-run")
		return

	# -a: Use archive suffix to determine the compression program.
	# -c: Create new.
	# -f: Specify file name.
	# --preserve-permissions(-p) and --same-owner are default for superuser.
	with ar_dst.open(mode="wb") as fout:
		os.fchmod(fout.fileno(), 0o600)
		cmd = [
			"tar",
			"-C", str(rsync_dst),
			"-acf", str(ar_dst), "."]
		exec(cmd)
	print()

def symlink(dst: pathlib.Path, ar_filename: str, dry_run: bool):
	print("symlink...")
	if dry_run:
		print("skip by dry-run")
		return

	linkpath = dst / LATEST_SLINK_NAME
	print(f"symlink {str(linkpath)} to {ar_filename}")
	linkpath.unlink(missing_ok=True)
	linkpath.symlink_to(ar_filename)
	print()

def main():
	parser = argparse.ArgumentParser(description="Auto backup script")
	parser.add_argument("src", help="backup source root")
	parser.add_argument("dst", help="backup destination root")
	parser.add_argument("--tag", action="store", help="prefix for archive file")
	parser.add_argument("--mount-check", action="store", help="check if the specified path is a mountpoint")
	parser.add_argument("--keep-count", type=int, help="keep compressed files and delete the others")
	parser.add_argument("--reserved-size", type=int, help="delete old compressed files to allocate free area (GiB)")
	parser.add_argument("--exclude-from", action="append", default=[], help="check if dst is a mountpoint")
	parser.add_argument("--dump-command", action="store", default="mariadb-dump", help="DB dump command (default=mariadb-dump)")
	parser.add_argument("--db", action="store", help="database name (backup if specified)")
	parser.add_argument("-n", "--dry-run", action="store_true", help="rsync dry-run")
	args = parser.parse_args()

	user = getpass.getuser()
	host = platform.node()
	dt_now = datetime.datetime.now()
	dt_str = dt_now.strftime('%Y%m%d_%H%M%S')
	tag = ""
	if args.tag is not None:
		tag = "_" + args.tag + "_"

	src = pathlib.Path(args.src).resolve()
	dst = pathlib.Path(args.dst).resolve()
	rsync_dst = dst / RSYNC_DIR_NAME
	ar_name = f"{user}_{host}{tag}{dt_str}.{ARCHIVE_EXT}"
	ar_dst = dst / ar_name
	ex_list = list(map(lambda s: pathlib.Path(s).resolve(), args.exclude_from))
	print(f"Date: {dt_str}")
	print(f"SRC: {src}")
	print(f"DST: {dst}")
	print(f"RSYNC DST: {rsync_dst}")
	print(f"AR DST: {ar_dst}")
	print(f"Mount Check: {args.mount_check}")
	print(f"Keep Count: {args.keep_count}")
	print(f"Reserved Size: {args.reserved_size}")
	print(f"Exclude From: {list(map(str, ex_list))}")
	print(f"Dump Command: {args.dump_command}")
	print(f"DB: {args.db}")
	print(f"Dry Run: {args.dry_run}")
	print()

	# mountpoint check
	if args.mount_check is not None:
		mount_check(args.mount_check)
	rsync_dst.mkdir(parents=True, exist_ok=True)
	# delete old files
	if args.keep_count is not None:
		delete_old_files(dst, args.keep_count)
	if args.reserved_size is not None:
		allocate_size(dst, args.reserved_size << 30)
	# rsync src/ to RSYNC_DIR_NAME/
	rsync(src, rsync_dst, ex_list, args.dry_run)
	# DB dump (removed by rsync. should do after rsync.)
	if args.db is not None:
		dump_db(rsync_dst, args.dump_command, args.db, args.dry_run)
	# tar
	archive(rsync_dst, ar_dst, args.dry_run)
	# symlink
	symlink(dst, ar_name, args.dry_run)

	print("OK!")

if __name__ == '__main__':
	main()
