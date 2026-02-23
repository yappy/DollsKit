#!/usr/bin/env python3

import sys
import os
import datetime
import pathlib
import subprocess

# ------------------------------------------------------------------------------
# Config (can be overriden by ENVVAR)
BKUP_MP = pathlib.Path(os.environ.get("BKUP_MP", "/mnt/bkup"))
SRC_DIR = pathlib.Path(os.environ.get("SRC_DIR", "/"))
KEEP_COUNT = os.environ.get("KEEP_COUNT", "30")
# Cloud upload (rclone) settings
# Disabled if RCLONE_REMOTE is empty
RCLONE_REMOTE = os.environ.get("RCLONE_REMOTE", "pcloud_enc")
RCLONE_DST = os.environ.get("RCLONE_DST", f"{os.uname().nodename}/full")
RCLONE_KEEP_COUNT = os.environ.get("RCLONE_KEEP_COUNT", "5")
# ------------------------------------------------------------------------------

SELF_DIR = pathlib.Path(__file__).resolve().parent
SCRIPT_DIR = SELF_DIR / "bkup" / "src"

BKUP_ROOT = BKUP_MP / "full"
SYNC_DIR = BKUP_ROOT / "sync"
ARCHIVE_DIR = BKUP_ROOT / "archive"


def exec_cmd(cmd: list[str]):
    print("EXEC:", " ".join(map(str, cmd)))
    sys.stdout.flush()
    sys.stderr.flush()
    subprocess.run(list(map(str, cmd)), check=True)


def main():
    print("--------------------------------------------------------------------------------")
    print("START")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")

    # check if mount point is available
    exec_cmd(["mountpoint", str(BKUP_MP)])

    # ensure directories exist
    SYNC_DIR.mkdir(parents=True, exist_ok=True)
    ARCHIVE_DIR.mkdir(parents=True, exist_ok=True)

    # sync
    exec_cmd([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "sync",
        "--src", str(SRC_DIR),
        "--dst", str(SYNC_DIR),
        "--exclude-from", str(SELF_DIR / "bkup_exclude.txt"),
        "--force",
    ])

    # archive
    exec_cmd([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "archive",
        "--src", str(SYNC_DIR),
        "--dst", str(ARCHIVE_DIR),
    ])

    # clean
    exec_cmd([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "clean",
        "--dst", str(ARCHIVE_DIR),
        "--keep-count", KEEP_COUNT,
    ])

    # Upload to cloud if RCLONE_REMOTE is set
    if RCLONE_REMOTE:
        exec_cmd([
            sys.executable, str(SCRIPT_DIR / "bkup.py"),
            "cloud",
            "--src", str(ARCHIVE_DIR),
            "--remote", RCLONE_REMOTE,
            "--dst", RCLONE_DST,
        ])

        exec_cmd([
            sys.executable, str(SCRIPT_DIR / "bkup.py"),
            "cloudclean",
            "--remote", RCLONE_REMOTE,
            "--dst", RCLONE_DST,
            "--keep-count", RCLONE_KEEP_COUNT,
        ])

    print("--------------------------------------------------------------------------------")
    print("END")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")
    print()


if __name__ == "__main__":
    main()
