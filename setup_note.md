# 参考資料
Raspberry Pi Documentation:
https://www.raspberrypi.org/documentation/

Headless と書かれた節を参考にすればディスプレイやキーボードやマウスなしでセットアップできる。

## 最終実践環境
Raspbian Buster Lite


# 準備
https://www.raspberrypi.org/documentation/configuration/wireless/headless.md

- raspbian の img を落とす
- balenaEtcher を落とす (全 OS でこれが推奨になったらしい)
- microSD に焼く
  - 128 GB 以上のに焼いたら警告が出た。。
- エクスプローラ等で FAT32 のブートパーティションを開き、`ssh`という名前の空ファイルを作る
- (無線を使う場合)
  - `wpa_supplicant.conf`という名前のファイルを作り、例に従って country, ssd, pass を書く
  - 保存後いきなり抜いたりしないこと
- メモカを本体に入れてから電源をつなぐ
- マウスと HDMI をつないでネットワーク設定を見るか、なんかほかの方法で IP addr を特定する
  - DHCP のアドレス範囲の先頭に現在つながっている機器の数を足した付近に対して ping, ssh して試す
- `ssh pi@<IP addr>`
  - user, pass = pi, raspberry
  - これで入れたら当たり


# 初期設定
`$sudo raspi-config`

- (1) Change User Password
  - パスワードを raspberry から変更する
- (2) Network Options
  - Hostname
  - Wifi
    - 国と SSID を聞かれる
- (4) Localisation Options
  - Change Locale
    - 日本語にしたいなら `ja_JP.UTF-8 UTF-8`
    - システムデフォルトに設定するといつもの出力がいろいろ変わる
  - Change Timezone
    - 東京に
  - Change Wi-fi Country
    - JP Japan にしておく
- (7) Advanced Options
  - Expand Filesystem
    - SDカード全体を使うようにする
    - 自動で行われるようになったっぽい
	- 確認は `df -h`

`ifconfig` で wlan の MAC addr を見てルータに DHCP 固定割り当てを設定する。
または普通の Linux のやり方で固定アドレスを設定する。


# アップデート
## 日本のミラーサイト
`/etc/apt/sources.list` に書かれているサーバは遠くて遅いので
以下のうちどれかに差し替える。

http://raspbian.org/RaspbianMirrors
* http://ftp.jaist.ac.jp/raspbian/
* http://ftp.tsukuba.wide.ad.jp/Linux/raspbian/raspbian/
* http://ftp.yz.yamagata-u.ac.jp/pub/linux/raspbian/raspbian/

なんか `/etc/apt/sources.list.d/raspi.list` も増えてるみたいだがこちらのミラーは謎。

## パッケージの更新
* `sudo apt update`
* `sudo apt upgrade`

## Raspberry Piのファームウェア更新(危険)
`sudo rpi-update`

開発中の最新版になるので注意

## パッケージの削除
`sudo apt remove --purge <PKGNAME>` or `sudo apt purge <PKGNAME>`

## 設定ファイルを後から消す
`` dpkg --purge `dpkg --get-selections | grep deinstall | cut -f1` ``


# 自動アップデート
1. `sudo apt install unattended-upgrades`
1. `sudo dpkg-reconfigure -plow unattended-upgrades`
1. `/etc/apt/apt.conf.d/50unattended-upgrades` を編集

このあたりを運用に合わせて設定する。
```
// Do automatic removal of new unused dependencies after the upgrade
// (equivalent to apt-get autoremove)
Unattended-Upgrade::Remove-Unused-Dependencies "true";

// Automatically reboot *WITHOUT CONFIRMATION* if
//  the file /var/run/reboot-required is found after the upgrade
Unattended-Upgrade::Automatic-Reboot "true";
```


# screen
- nohup だと ssh が切れた後プロセスが死んでしまう(原因は不明)
- `sudo apt-get install screen`
  - デタッチ: C-a d


# ssh をまともにする
- root password
  - `sudo passwd root`
  - 最終的には /etc/shadow で * にしておく方がよいかも
- ssh 設定
  - `sudo vi /etc/ssh/sshd_config`
  - Change ssh port
    - Port 22 <-変える
  - disable root login
    - PermitRootLogin no
  - パスワード認証の無効化
    - `#PasswordAuthentication yes`
  - pi の ssh ログインを不許可 (他にログイン可能な sudoer がいることを確認してから！)
    - `DenyUsers pi`

# VNC (remote desktop)
https://www.raspberrypi.org/documentation/remote-access/vnc/


# カメラモジュール
- カメラモジュールの有効化
  - sudo raspi-config
  - Enable Camera

- 静止画撮影(要 video グループ)
  - `raspistill -t 1 -o pic.jpg`
  - -t 指定すると真っ黒になってしまうことがあるらしい？
    https://mizukama.sakura.ne.jp/blog/archives/4022

- サイズ
  - 3280x2464
- exif のサムネイル
  - 64x48

- C# からサムネイルの取り出し
  - GetThumbnailImage()


# HTTP Server
- lighttpd
  - `sudo apt-get install lighttp`
- php
  - `sudo apt-get install php-cgi`

- lighttpd
  - `/etc/lighttpd`
  - `lighttpd-enable-mod`, `lighttpd-disable-mod`
  - `service lighttpd force-reload`
  - 以下などを必要に応じて変更しながら enable する
    - accesslog
	- userdir
	- fastcgi
	- fastcgi-php
  - CGI の stderr
    - `server.breakagelog = "/var/log/lighttpd/breakagelog.log"`


# SSL (Let's Encrypt)
- `sudo apt install certbot`
  - backports でなくても入っているみたい。
  - ただしバージョンはかなり古い模様。(certbot 0.10.2)

`sudo certbot`
```
Certbot doesn't know how to automatically configure the web server on this system. However, it can still get a certificate for you. Please run "certbot certonly" to do so. You'll need to manually configure your web server to use the resulting certificate.
```
`sudo certbot certonly`
```
How would you like to authenticate with the ACME CA?
-------------------------------------------------------------------------------
1: Place files in webroot directory (webroot)
2: Spin up a temporary webserver (standalone)
-------------------------------------------------------------------------------
Select the appropriate number [1-2] then [enter] (press 'c' to cancel): 1
Enter email address (used for urgent renewal and security notices) (Enter 'c' to
cancel): (メール)

-------------------------------------------------------------------------------
Please read the Terms of Service at
https://letsencrypt.org/documents/LE-SA-v1.2-November-15-2017.pdf. You must
agree in order to register with the ACME server at
https://acme-v01.api.letsencrypt.org/directory
-------------------------------------------------------------------------------
(A)gree/(C)ancel: a
Please enter in your domain name(s) (comma and/or space separated)  (Enter 'c'
to cancel): (ドメイン名)
Obtaining a new certificate
Performing the following challenges:
http-01 challenge for (ドメイン名)

Select the webroot for (ドメイン名):
-------------------------------------------------------------------------------
1: Enter a new webroot
-------------------------------------------------------------------------------
Press 1 [enter] to confirm the selection (press 'c' to cancel): 1
Input the webroot for yappy.mydns.jp: (Enter 'c' to cancel):/var/www/html/
(lighttpd のデフォルトドキュメントルートの場合; tab 補完が効く)
Waiting for verification...
Cleaning up challenges
Generating key (2048 bits): /etc/letsencrypt/keys/0000_key-certbot.pem
Creating CSR: /etc/letsencrypt/csr/0000_csr-certbot.pem

IMPORTANT NOTES:
 - Congratulations! Your certificate and chain have been saved at
   /etc/letsencrypt/live/(ドメイン名)/fullchain.pem. Your cert will
   expire on 2019-01-01. To obtain a new or tweaked version of this
   certificate in the future, simply run certbot again. To
   non-interactively renew *all* of your certificates, run "certbot
   renew"
 - If you like Certbot, please consider supporting our work by:

   Donating to ISRG / Let's Encrypt:   https://letsencrypt.org/donate
   Donating to EFF:                    https://eff.org/donate-le
```

- 秘密鍵と証明書を結合する。
  - `sudo -sE`
  - `cd /etc/letsencrypt/live/(ドメイン)`
  - `cat privkey.pem cert.pem > server.pem`
- lighttpd に設定する。
  - /etc/lighttpd/conf-available/10-ssl.conf をコピーして使う。
  - セキュアな設定は https://cipherli.st/ がよい。
  - `sudo lighttpd-enable-mod (xx- と .conf を除いた名前)`
  - `sudo service lighttpd force-reload`
```
ssl.pemfile = "/etc/letsencrypt/live/yappy.mydns.jp/ssl.pem"
ssl.ca-file = "/etc/letsencrypt/live/yappy.mydns.jp/fullchain.pem"
```


# MySQL (not used now)
- `apt-get install mysql-server`
- Debian 9 では中身は MariaDB になっている。

## Ruby (Mysql2) (not used now)
MySQL クライアント用の C ライブラリが必要。
default-libmysqlclient-dev っていうのが MySQL 用と互換性高そうな MariaDB 用ライブラリへの依存になってた。
こいつを追いかけておくのがよさそう。
> dep: libmariadbclient-dev-compat
>    MariaDB database development files (libmysqlclient compatibility) 

Ruby ネイティブ拡張用 C ライブラリが必要。
```
sudo apt-get install default-libmysqlclient-dev
sudo apt-get install ruby-dev
sudo gem install mysql2
```
