# 参考資料
Raspberry Pi Documentation:
https://www.raspberrypi.org/documentation/

Headless と書かれた節を参考にすればディスプレイやキーボードやマウスなしでセットアップできる。

# 準備
- raspbian の img を落とす
- Etcher を落とす (全 OS でこれが推奨になったらしい)
- microSD に焼く
- エクスプローラ等で開き、`ssh`という名前の空ファイルを作る
- メモカを本体に入れてから電源をつなぐ
  - あと有線 LAN も
- マウスと HDMI をつないでネットワーク設定を見るか、なんかほかの方法で IP addr を特定する
  - DHCP のアドレス範囲の先頭に現在つながっている機器の数を足した付近に対して ping, ssh して試す
- `ssh <IP addr>`
  - user, pass = pi, raspberry
  - これで入れたら当たり


# 初期設定
`$sudo raspi-config`
- (1) Expand Filesystem
  - SDカード全体を使うようにする
- (2) Change User Password
  - パスワードを raspberry から変更する
- (4) Internationalization Options
  - タイムゾーンは東京にしておいたほうがいいかも
  - wifi は日本にしておいたほうがいいかも
    - (変な設定にすると法規制に触れたりする？？？)
- (9) Advanced Options
  - hostname とか
  - raspi-config 自体のアップデートが一番下にある


# 無線
忘れた。ググってなんかうまくやる。


# アップデート
## 日本のミラーサイト
http://raspbian.org/RaspbianMirrors
* http://ftp.jaist.ac.jp/raspbian/
* http://ftp.tsukuba.wide.ad.jp/Linux/raspbian/raspbian/
* http://ftp.yz.yamagata-u.ac.jp/pub/linux/raspbian/raspbian/

## パッケージの更新
* `sudo apt-get update`
* `sudo apt-get upgrade`
* `sudo apt-get dist-upgrade`

## Raspberry Piのファームウェア更新(危険)
`sudo rpi-update`

開発中の最新版になるので注意

## パッケージの削除
`sudo apt-get remove --purge <PKGNAME>` or `sudo apt-get purge <PKGNAME>`

## 設定ファイルを後から消す
`` dpkg --purge `dpkg --get-selections | grep deinstall | cut -f1` ``


# Mono (C#)
- mono は古い
- 公開鍵取得
  - `sudo apt-key adv --keyserver keyserver.ubuntu.com --recv-keys 3FA7E0328081BFF6A14DA29AA6A19B38D3D831EF`
- サーバー追加
  - `echo "deb http://download.mono-project.com/repo/debian wheezy main" | sudo tee /etc/apt/sources.list.d/mono-xamarin.lis`
- update
  - `sudo apt-get update`
- install とか upgrade とか擦るといける
  - `sudo apt-get update`
  - `sudo apt-get install mono-complete`
- nuget もこれでいける(多分)
  - `sudo apt-get install nuget`
- 自己アップデート
  - `sudo nuget update -self`


# screen
- nohup だと ssh 切れた後プロセスが死んでしまう(原因は不明)
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
  - 公開鍵認証
    - AuthorizedKeysFile  %h/.ssh/authorized_keys
  - パスワード認証の無効化
    - `#PasswordAuthentication yes`

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


# 形態素解析
- MeCab
  - `sudo apt-get install mecab mecab-ipadic-utf8`


# HTTP Server
- lighttpd
  - `sudo apt-get install lighttp`
- php
  - `sudo apt-get install php-cgi`

- lighttpd
  - CGI の stderr
    - `server.breakagelog = "/var/log/lighttpd/stderr.log"`


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
  - `cat privkey.pem ccert.pem > ssl.pem`
- lighttpd に設定する。
  - /etc/lighttpd/conf-available/10-ssl.conf をコピーして使う。
  - セキュアな設定は https://cipherli.st/ がよい。
  - `sudo lighttpd-enable-mod (xx- と .conf を除いた名前)`
  - `sudo service lighttpd force-reload`
```
ssl.pemfile = "/etc/letsencrypt/live/yappy.mydns.jp/ssl.pem"
ssl.ca-file = "/etc/letsencrypt/live/yappy.mydns.jp/fullchain.pem"
```


# MySQL
- `apt-get install mysql-server`
- Debian 9 では中身は MariaDB になっている。

## Ruby (Mysql2)
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
