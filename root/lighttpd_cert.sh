#!/bin/sh

# Reload the updated certificate file
# Copy to:
# /etc/cron.weekly/

/usr/sbin/service lighttpd force-reload
