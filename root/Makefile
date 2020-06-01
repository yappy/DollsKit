# cd /etc/letsencrypt/live/<domain>
# ln -s /path/to/this/Makefile
# cron: make -C /etc/letsencrypt/live/<domain>

# let's encrypt
# ssl.pemfile = "/etc/letsencrypt/live/<domain>/server.pem"
# ssl.ca-file = "/etc/letsencrypt/live/<domain>/fullchain.pem"

all: server.pem

server.pem: privkey.pem cert.pem
	cat privkey.pem cert.pem > server.pem
