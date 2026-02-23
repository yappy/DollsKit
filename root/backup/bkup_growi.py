#!/usr/bin/env python3

import sys
import os
import datetime
import pathlib
import shutil
import subprocess

# ------------------------------------------------------------------------------
# Config (can be overriden by ENVVAR)
BKUP_MP = pathlib.Path(os.environ.get("BKUP_MP", "/mnt/bkup"))
KEEP_COUNT = os.environ.get("KEEP_COUNT", "30")
PROJS = os.environ.get("PROJS", "growi").split()
# Cloud upload (rclone) settings: set RCLONE_REMOTE to enable upload
# Disabled if RCLONE_REMOTE is empty
RCLONE_REMOTE = os.environ.get("RCLONE_REMOTE", "pcloud_enc")
RCLONE_DST = os.environ.get("RCLONE_DST", f"{os.uname().nodename}/growi")
RCLONE_KEEP_COUNT = os.environ.get("RCLONE_KEEP_COUNT", "30")
# ------------------------------------------------------------------------------

SELF_DIR = pathlib.Path(os.path.dirname(__file__))
SCRIPT_DIR = SELF_DIR / "bkup" / "src"
BKUP_ROOT = BKUP_MP / "growi"
DUMP_DIR = BKUP_ROOT / "dump"
ARCHIVE_DIR = BKUP_ROOT / "archive"
ARCHIVE_TAG = "growi"
# target GROWI service (DB)
SERVICE = "mongo"


def exec(cmd: list[str]):
    print("EXEC:", " ".join(cmd))
    sys.stdout.flush()
    sys.stderr.flush()
    subprocess.run(cmd, check=True)


# mongodump > /tmp/{proj}.archive (in the container)
# docker compose cp to the host
def dbdump(proj: str):
    ar_path_cont = f"/tmp/{proj}.archive"
    ar_path_host = str(DUMP_DIR / f"{proj}.archive")
    exec([
        "docker", "compose", "-p", proj, "exec", SERVICE,
        "mongodump", "--quiet", f"--archive={ar_path_cont}"
    ])
    exec([
        "docker", "compose", "-p", proj, "cp",
        f"{SERVICE}:{ar_path_cont}", ar_path_host
    ])
    exec([
        "docker", "compose", "-p", proj, "exec", SERVICE,
        "rm", ar_path_cont
    ])


def archive():
    exec([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "archive",
        "--src", str(DUMP_DIR),
        "--dst", str(ARCHIVE_DIR),
        "--tag", ARCHIVE_TAG,
    ])


def clean():
    exec([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "clean",
        "--dst", str(DUMP_DIR),
        "--keep-count", KEEP_COUNT,
    ])


def cloud():
    exec([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "cloud",
        "--src", str(ARCHIVE_DIR),
        "--remote", RCLONE_REMOTE,
        "--dst", RCLONE_DST,
    ])


def cloud_clean():
    exec([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "cloudclean",
        "--remote", RCLONE_REMOTE,
        "--dst", RCLONE_DST,
        "--keep-count", RCLONE_KEEP_COUNT,
    ])


def main():
    print("--------------------------------------------------------------------------------")
    print("START")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")

    # check if the mount point is available
    exec(["mountpoint", str(BKUP_MP)])
    # clean dump dir and mkdir
    shutil.rmtree(DUMP_DIR, ignore_errors=True)
    DUMP_DIR.mkdir(parents=True, exist_ok=True)
    ARCHIVE_DIR.mkdir(parents=True, exist_ok=True)

    # main
    for proj in PROJS:
        dbdump(proj)
    archive()
    clean()
    if RCLONE_REMOTE:
        cloud()
        cloud_clean()

    print("--------------------------------------------------------------------------------")
    print("END")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")
    print()


if __name__ == "__main__":
    main()
