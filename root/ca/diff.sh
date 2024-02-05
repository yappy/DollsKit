#!/bin/bash
set -v
set -ue -o pipefail

diff ./CA.pl /usr/lib/ssl/misc/CA.pl
diff ./openssl.cnf /usr/lib/ssl/openssl.cnf
