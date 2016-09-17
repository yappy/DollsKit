#!/bin/bash
#set -x
set -ue -o pipefail

# USB memory mountpoint
workdir="/media/usbbkup"
# backup source dir
rsyncsrc="/"
# backup destination dir
rsyncdst="${workdir}/backup"
# backup archive file path
ardst="${workdir}/bkup"`date +%Y%m%d_%H%M%S`".tar.bz2"
# backup archive limit (days)
arlimit="+30"

# exclude
exc=""
exc="${exc} --exclude=/proc"
exc="${exc} --exclude=/sys"
exc="${exc} --exclude=/dev"
exc="${exc} --exclude=/boot"
exc="${exc} --exclude=/run"
exc="${exc} --exclude=/lost+found"
exc="${exc} --exclude=/media"
exc="${exc} --exclude=/mnt"
exc="${exc} --exclude=/tmp"
exc="${exc} --exclude=/var/tmp"
exc="${exc} --exclude=/etc/fstab"
exc="${exc} --exclude=/etc/recolv.conf"
exc="${exc} --exclude=/var/log"

echo "Backup media mount check..."
mountpoint $workdir
echo "OK"

echo "Delete old backup archives..."
find  ${workdir} -maxdepth 1 -name "*.tar.bz2" -mtime ${arlimit} | \
    xargs rm -fv
echo "Complete!"

date

echo "rsync..."
rsync -aur --delete $exc $rsyncsrc $rsyncdst
echo "Complete!"

date

echo "Archive to ${ardst} ..."
tar -C $workdir -apcf $ardst $rsyncdstdir
echo "Complete!"

date
echo "Backup has completed successfully"
