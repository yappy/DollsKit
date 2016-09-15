#!/bin/bash
#set -x
set -ue -o pipefail

workdir="/media/usbbkup"
rsyncsrc="/"
rsyncdstdir="backup"
rsyncdst="${workdir}/${rsyncdstdir}"
ardst="${workdir}/bkup"`date +%Y%m%d_%H%M%S`".tar.bz2"

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

date

echo "rsync..."
rsync -aur --delete $exc $rsyncsrc $rsyncdst
echo "Complete!"

date

echo "archive to ${ardst} ..."
tar -C $workdir -apcf $ardst $rsyncdstdir
echo "Complete!"

date
echo "Backup has completed successfully"
