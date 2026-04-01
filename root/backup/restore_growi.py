#!/usr/bin/env python3

import argparse
import sys
import os
import pathlib
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


def exec_cmd(cmd: list[str]):
    print("EXEC:", " ".join(cmd))
    sys.stdout.flush()
    sys.stderr.flush()
    subprocess.run(cmd, check=True)


# mongorestore < /tmp/{proj}.archive (in the container)
# docker compose cp to the host
def restore(proj: str, archive_path: str):
    ar_path_cont = "/tmp/restore.archive"
    ar_path_host = archive_path
    exec_cmd([
        "docker", "compose", "-p", proj, "cp",
        ar_path_host, f"{SERVICE}:{ar_path_cont}"
    ])
    exec_cmd([
        "docker", "compose", "-p", proj, "exec", SERVICE,
        "mongorestore", "--verbose", f"--archive={ar_path_cont}"
    ])
    exec_cmd([
        "docker", "compose", "-p", proj, "exec", SERVICE,
        "rm", ar_path_cont
    ])


def main():
    parser = argparse.ArgumentParser(
        description="Restore GROWI data from backup (mongodump archive)",
    )
    parser.add_argument("--archive", "-a", required=True, help="mongodump archive file path")
    parser.add_argument("--project", "-p", required=True, help="docker compose project name")

    args = parser.parse_args()

    restore(args.project, args.archive)


if __name__ == "__main__":
    main()
