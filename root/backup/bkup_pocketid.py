#!/usr/bin/env python3

"""Create a consistent Pocket ID volume + encryption-key backup."""

import os
import pathlib
import shutil
import stat
import subprocess
import sys

BKUP_MP = pathlib.Path(os.environ.get("BKUP_MP", "/mnt/bkup"))
KEEP_COUNT = os.environ.get("KEEP_COUNT", "30")
RCLONE_REMOTE = os.environ.get("RCLONE_REMOTE", "pcloud_enc")
RCLONE_DST = os.environ.get("RCLONE_DST", f"{os.uname().nodename}/pocketid")
RCLONE_KEEP_COUNT = os.environ.get("RCLONE_KEEP_COUNT", "5")

SELF_DIR = pathlib.Path(__file__).resolve().parent
SCRIPT_DIR = SELF_DIR / "bkup" / "src"
ENV_FILE = pathlib.Path("/root/pocketid/pocketid.env")
SERVICE = "pocketid.service"
PROJECT = "pocket-id"
VOLUME_KEY = "pocket_id_data"
BKUP_ROOT = BKUP_MP / "pocketid"
DUMP_DIR = BKUP_ROOT / "dump"
ARCHIVE_DIR = BKUP_ROOT / "archive"
ARCHIVE_TAG = "pocketid"


def exec_cmd(cmd: list[str], *, capture: bool = False) -> str:
    print("EXEC:", " ".join(map(str, cmd)))
    sys.stdout.flush()
    result = subprocess.run(
        list(map(str, cmd)), check=True, text=True,
        stdout=subprocess.PIPE if capture else None,
    )
    return result.stdout.strip() if capture else ""


def find_volume() -> str:
    names = exec_cmd([
        "docker", "volume", "ls", "--quiet",
        "--filter", f"label=com.docker.compose.project={PROJECT}",
        "--filter", f"label=com.docker.compose.volume={VOLUME_KEY}",
    ], capture=True).splitlines()
    if len(names) != 1:
        raise RuntimeError(
            f"Expected exactly one Compose volume for {PROJECT}/{VOLUME_KEY}, found {len(names)}"
        )
    return names[0]


def dump(volume: str):
    DUMP_DIR.mkdir(mode=0o700, parents=True, exist_ok=True)
    DUMP_DIR.chmod(0o700)
    exec_cmd([
        "docker", "run", "--rm",
        "--volume", f"{volume}:/source:ro",
        "--volume", f"{ENV_FILE}:/secrets/pocketid.env:ro",
        "--volume", f"{DUMP_DIR}:/dump",
        "busybox:latest", "sh", "-c",
        "umask 077 && mkdir /dump/data && cp -a /source/. /dump/data/ "
        "&& cp -p /secrets/pocketid.env /dump/pocketid.env",
    ])


def archive():
    exec_cmd([
        sys.executable, str(SCRIPT_DIR / "bkup.py"), "archive",
        "--src", str(DUMP_DIR), "--dst", str(ARCHIVE_DIR),
        "--tag", ARCHIVE_TAG,
    ])


def clean():
    exec_cmd([
        sys.executable, str(SCRIPT_DIR / "bkup.py"), "clean",
        "--dst", str(ARCHIVE_DIR), "--keep-count", KEEP_COUNT,
    ])


def cloud():
    exec_cmd([
        sys.executable, str(SCRIPT_DIR / "bkup.py"), "cloud",
        "--src", str(ARCHIVE_DIR), "--remote", RCLONE_REMOTE,
        "--dst", RCLONE_DST,
    ])


def cloud_clean():
    exec_cmd([
        sys.executable, str(SCRIPT_DIR / "bkup.py"), "cloudclean",
        "--remote", RCLONE_REMOTE, "--dst", RCLONE_DST,
        "--keep-count", RCLONE_KEEP_COUNT,
    ])


def main():
    if not ENV_FILE.is_file():
        raise FileNotFoundError(f"Pocket ID environment file not found: {ENV_FILE}")
    if stat.S_IMODE(ENV_FILE.stat().st_mode) & 0o077:
        raise PermissionError(f"{ENV_FILE} must not be readable by group or others")
    exec_cmd(["mountpoint", str(BKUP_MP)])
    volume = find_volume()

    shutil.rmtree(DUMP_DIR, ignore_errors=True)
    DUMP_DIR.mkdir(mode=0o700, parents=True)
    ARCHIVE_DIR.mkdir(mode=0o700, parents=True, exist_ok=True)
    ARCHIVE_DIR.chmod(0o700)

    was_active = subprocess.run(
        ["systemctl", "is-active", "--quiet", SERVICE], check=False
    ).returncode == 0
    stopped = False
    try:
        if was_active:
            exec_cmd(["systemctl", "stop", SERVICE])
            stopped = True
        dump(volume)
    finally:
        if stopped:
            exec_cmd(["systemctl", "start", SERVICE])

    archive()
    clean()
    if RCLONE_REMOTE:
        cloud()
        cloud_clean()


if __name__ == "__main__":
    main()
