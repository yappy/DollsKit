#!/usr/bin/env python3

import os
import datetime
import pathlib
import shutil
import subprocess

# ------------------------------------------------------------------------------
# Config (can be overriden by ENVVAR)
BKUP_MP = pathlib.Path(os.environ.get("BKUP_MP", "/mnt/bkup"))
KEEP_COUNT = int(os.environ.get("KEEP_COUNT", "30"))
PROJS = os.environ.get("PROJS", "growi-public growi-private").split()
# ------------------------------------------------------------------------------

SELF_DIR = pathlib.Path(os.path.dirname(__file__))
SCRIPT_DIR = SELF_DIR / "bkup" / "src"
BKUP_ROOT = BKUP_MP / "growi"
BKUP_DUMP = BKUP_ROOT / "dump"
BKUP_ARCHIVE = BKUP_ROOT / "archive"
# target GROWI service (DB)
SERVICE = "mongo"


def exec(cmd: list[str]):
    print("EXEC:", " ".join(cmd))
    subprocess.run(cmd, check=True)


def dbdump():
    pass


def main():
    # check if the mount point is available
    exec(["mountpoint", str(BKUP_MP)])

    shutil.rmtree(BKUP_DUMP, ignore_errors=True)
    BKUP_DUMP.mkdir(parents=True, exist_ok=True)
    BKUP_ARCHIVE.mkdir(parents=True, exist_ok=True)

    print("--------------------------------------------------------------------------------")
    print("START")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")

    for proj in PROJS:
        ar_path_cont = f"/tmp/{proj}.archive"
        ar_path_host = str(BKUP_DUMP / f"{proj}.archive")
        exec(["docker", "compose", "-p", proj, "exec", SERVICE,
              "mongodump", f"--archive={ar_path_cont}"])
        exec(["docker", "compose", "-p", proj, "cp",
              f"{SERVICE}:{ar_path_cont}", ar_path_host]),

        # python3 "${SCRIPT_DIR}/bkup.py" \
        # clean \
        # --dst "${ARCHIVE_DIR}" \
        # --keep-count "${KEEP_COUNT}"

    print("--------------------------------------------------------------------------------")
    print("END")
    print(datetime.datetime.now())
    print("--------------------------------------------------------------------------------")
    print()


if __name__ == "__main__":
    main()
