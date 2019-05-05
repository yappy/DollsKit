#!/bin/bash
set -v
set -ue -o pipefail

# essential tools
apt install -y git build-essential cmake

# C library
apt install -y libssl1.0-dev libcurl4-openssl-dev libmicrohttpd-dev

# HTTP server + ssl/tls
apt install -y lighttpd certbot
apt install -y php-cgi
