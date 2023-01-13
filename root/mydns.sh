#!/bin/sh

# Create a file: /root/mydns
# user:pass
# permission recommended: 400

# Copy to:
# /etc/cron.daily/

HOME=/root
curl -u `cat $HOME/mydns` $MYDNS https://www.mydns.jp/login.html
