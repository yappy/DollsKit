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
PROJS = os.environ.get("PROJS", "growi-public growi-private").split()
# ------------------------------------------------------------------------------

SELF_DIR = pathlib.Path(os.path.dirname(__file__))
SCRIPT_DIR = SELF_DIR / "bkup" / "src"
BKUP_ROOT = BKUP_MP / "growi"
DUMP_DIR = BKUP_ROOT / "dump"
ARCHIVE_DIR = BKUP_ROOT / "archive"
# target GROWI service (DB)
SERVICE = "mongo"


def exec(cmd: list[str]):
    print("EXEC:", " ".join(cmd))
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


def archive():
    exec([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "archive",
        "--src", str(DUMP_DIR),
        "--dst", str(ARCHIVE_DIR),
    ])


def clean():
    exec([
        sys.executable, str(SCRIPT_DIR / "bkup.py"),
        "clean",
        "--dst", str(DUMP_DIR),
        "--keep-count", KEEP_COUNT,
    ])


def main():
    # check if the mount point is available
    exec(["mountpoint", str(BKUP_MP)])

    shutil.rmtree(DUMP_DIR, ignore_errors=True)
    DUMP_DIR.mkdir(parents=True, exist_ok=True)
    ARCHIVE_DIR.mkdir(parents=True, exist_ok=True)

    print("--------------------------------------------------------------------------------")
    print("START")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")

    for proj in PROJS:
        dbdump(proj)
    archive()
    clean()

    print("--------------------------------------------------------------------------------")
    print("END")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")
    print()


if __name__ == "__main__":
    main()
