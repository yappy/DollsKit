#!/bin/sh

# Replace <domain> and copy to:
# /etc/cron.weekly/

make -C /etc/letsencrypt/live/<domain>
service lighttpd force-reload
