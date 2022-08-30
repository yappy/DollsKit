#!/bin/sh

# Replace <domain> and copy to:
# /etc/cron.weekly/

/usr/bin/make -C /etc/letsencrypt/live/<domain>
/usr/sbin/service lighttpd force-reload
