CREATE USER 'www-data' @'localhost' IDENTIFIED VIA unix_socket;

CREATE DATABASE wordpress CHARACTER SET utf8mb4 COLLATE utf8mb4_bin;

GRANT ALL ON wordpress.* to 'www-data' @'localhost';
