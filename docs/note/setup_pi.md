# 一日でなれる人形遣い

## 用意(購入)するもの

Optional でないものは必須です。

本体

* Raspberry Pi 4 Model B
  * Memory 1/2/4/8 GB
    * 基本的にその時点で調べて一番新しく一番メモリの多いモデルを選べば OK。(要審議)
* 電源 (家庭用 AC > USB type-C)
  * 給電は最近 type-C になった。
    消費電力が高くなりがちなので純正品を推奨。
* Micro SD カード
  * これをハードディスク (最近は SSD か) の代わり的に使う。
    つまり容量が大きくて読み書きが速いものを選べば OK。
* ケース (Optional)
  * そのままだと基板がむき出しになって埃をかぶるので購入推奨。
* 専用カメラモジュール (Optional)

その他

* SD カードリーダ/ライタ
  * PC から SD カードを読み書きする環境。
    PC に最初からついているならいいが、ないなら外付けを購入する。
    ノート PC だとついていがち。
* 有線/無線ルータ
  * 有線の場合は LAN ケーブルも。
* モニタ + Micro HDMI ケーブル (Optional)
  * 片方が小さいケーブルでないとディスプレイにつながらないので注意。
  * これ及び以降は初心者向け (本文書では取り扱わないため罠があるかもしれないので注意)
* マウス (Optional)
* キーボード (Optional)

## 参考資料

Raspberry Pi Documentation:
<https://www.raspberrypi.org/documentation/>

Getting Started
<https://www.raspberrypi.com/documentation/computers/getting-started.html>

Headless Setup (GUI なしセットアップ)
<https://www.raspberrypi.com/documentation/computers/configuration.html#set-up-a-headless-raspberry-pi>

ドキュメントが一新され、Raspberry Pi Imager を使ったセットアップ方法となった。
SD card に焼くイメージのセレクタ/ダウンローダとイメージライタが一緒になって
いい感じになったセットアップ用ソフト。

## 最新確認環境

### RPi5

Raspberry Pi OS Lite (64-bit)
2025-12-04
(Trixie)

### RPi4

Raspberry Pi OS Lite (32-bit)
2021-05-07
(Buster)

## 準備

<https://www.raspberrypi.com/documentation/computers/getting-started.html>

* Using Raspberry Pi Imager から Raspberry Pi Imager を落とす。
* Micro SD を挿入し、イメージと書き込み先を選択する。
  * GUI を使わない場合は Lite (no desktop) で OK。
* Raspberry Pi Imager v.2.0.0 では SSH や wifi は普通に設定できる。
* ~~`Ctrl + Shift + X` で Advanced Options を開く。~~
  * ~~ソフト内には説明がなく、公式ドキュメントを読んだ者のみが使える隠しコマンド。~~
  * SSH や wifi の設定をここからできる。
    この時点で公開鍵 SSH にもできる。
    (従来の SD root に特定のファイルを置く方法を行っていると思われる)
  * user:pass = pi:raspberry はセキュリティ上の理由で廃止方向。
* SD card を本体に入れてから電源をつなぐ。
* マウスと HDMI をつないでネットワーク設定を見るか、なんらかの他の方法で
  IP address を特定する。
  * DHCP のアドレス範囲の先頭に現在つながっている機器の数を足した付近に対して
    ping, ssh して試す。ssh TCP 22 番ポートが開いているはず。
  * `ssh <user>@<IP addr>` から設定したパスワードで入れたら当たり。
  * 最近のルータは HTTP 設定画面で接続中のデバイス一覧が見られることも多い。

## 初期設定

`$sudo raspi-config`

バージョンアップで少しずつパワーアップしている気がする。
Raspberry Pi Imager の時点で設定可能なものも増えてきている気がする。

* (1) System Options
  * wifi 設定
  * initial user のパスワード設定
  * Network at Boot
    * 新機能？ネットワークに接続するまでブートを待たせるらしい。
    * 以前は起動時に立ち上げたプログラムがネットワークエラーを起こしていたので
      オススメかも。
* (5) Localisation Options
* (6) Advanced Options
  * Expand Filesystem
    * SDカード全体を使うようにする
    * 自動でいつの間にか行われるようになったっぽい
  * 確認は `df -h`
* (8) Update
  * このツールをアップデートする

`ifconfig` で wlan の MAC addr を見てルータに DHCP 固定割り当てを設定する。
(そのような機能がある場合)
または普通の Linux のやり方で固定アドレスを設定する。
Windows で `ipconfig.exe /all` を実行した結果を参考にするとよい。

```text
# /etc/dhcpcd.conf
interface <eth0|wlan0>
static ip_address=192.168.XXX.YYY/NN
static routers=192.168.0.1
static domain_name_servers=XXX.YYY.ZZZ.WWW
```

## アップデート

### 日本のミラーサイト

※これをやらなくても tsukuba.wide.ad.jp につながった。
謎の力で近くのミラーが使われるようになったのかもしれない。

`/etc/apt/sources.list` に書かれているサーバは遠くて遅いので
以下のうちどれかに差し替える。

<http://raspbian.org/RaspbianMirrors>

* <http://ftp.jaist.ac.jp/raspbian/>
* <http://ftp.tsukuba.wide.ad.jp/Linux/raspbian/raspbian/>
* <http://ftp.yz.yamagata-u.ac.jp/pub/linux/raspbian/raspbian/>

### パッケージの更新

* `sudo apt update`
* `sudo apt upgrade`

### パッケージの削除

`sudo apt remove --purge <PKGNAME>` or `sudo apt purge <PKGNAME>`

### 設定ファイルを後から消す

`` dpkg --purge `dpkg --get-selections | grep deinstall | cut -f1` ``

## Debian-Backports

主にgit や cmake が古い場合。気にならないならスキップで OK。
最新を追いかけるなら公式のリポジトリを sources.list に登録するのが確実だが、
こちらで十分なら設定は一回で済む。

以下を `/etc/apt/sources.list` に追加。

```text
deb http://ftp.jp.debian.org/debian buster-backports main contrib non-free
```

その後 `sudo apt update`。
おそらく鍵エラーが出るので、NO_PUBKEY と言われた鍵(16進)を控えて、

```text
sudo apt-key adv --keyserver keyserver.ubuntu.com --recv-keys <PUBKEY>
...
```

`apt show` で backports に別バージョンがあるなら
`-a` ですべて表示できると注意が出る。
backports は `-t` で明示的に指定しなければ使われることはない。

```sh
apt show -a <pkg>
apt install -t <version>-backports <pkg>
```

## 自動補完の Beep 音がうるさい

```sh
$ sudo nano /etc/inputrc
# uncomment
set bell-style none
```

## Bash のタブ補完

デフォルトで入ってるのか入ってないのかはっきりしない。

```sh
apt install bash-completion
```

### root でタブ補完が効かない

一般ユーザの `.bashrc` では以下のコードで有効化されていて、
全員共通の `/etc/bashrc` ではコメントアウトされている？？？
よくわかんないけど共通設定ファイルをコメントアウト解除するか
/root/.bashrc の最後にコピペする。

```bash
# enable programmable completion features (you don't need to enable
# this, if it's already enabled in /etc/bash.bashrc and /etc/profile
# sources /etc/bash.bashrc).
if ! shopt -oq posix; then
  if [ -f /usr/share/bash-completion/bash_completion ]; then
    . /usr/share/bash-completion/bash_completion
  elif [ -f /etc/bash_completion ]; then
    . /etc/bash_completion
  fi
fi
```

## 自動アップデート

1. `sudo apt install unattended-upgrades`
1. `sudo dpkg-reconfigure -plow unattended-upgrades`
1. `/etc/apt/apt.conf.d/50unattended-upgrades` を編集

初期設定は以下のようになっているが、Raspberry Pi では origin が
"Raspbian" や "Raspberry Pi Foundation" になっているのでこのままではマッチせず
何もアップデートされない。

```text
"origin=Debian,codename=${distro_codename},label=Debian";
"origin=Debian,codename=${distro_codename},label=Debian-Security";
```

origin, label, suite 等の情報は `/var/lib/apt/lists/` 以下にあるファイルに
書かれているが、いつも `apt update` `apt upgrade` だけしているなら
以下のようにすればとりあえず全部アップデートできる。

```text
"o=*";
```

このあたりを運用に合わせて設定する。

```text
// Do automatic removal of new unused dependencies after the upgrade
// (equivalent to apt-get autoremove)
Unattended-Upgrade::Remove-Unused-Dependencies "true";

// Automatically reboot *WITHOUT CONFIRMATION* if
//  the file /var/run/reboot-required is found after the upgrade
Unattended-Upgrade::Automatic-Reboot "true";
```

以下で空実行できる。

```sh
sudo unattended-upgrade --debug --dry-run
```

## screen

* nohup だと ssh が切れた後プロセスが死んでしまう(原因は不明)
* `sudo apt install screen`
  * デタッチ: C-a d

## ssh をまともにする

* `sudo vi /etc/ssh/sshd_config`
* `sshd_config.d/` 以下にファイルを置いて include する方式に変わった気もする。
  * 放置していると接続が切れる
    * 以下をコメントアウト解除して設定する
      * `ClientAliveInterval 60`
      * `ClientAliveCountMax 3`
  * Change ssh port
    * `Port 22` <- 変える
  * Disable root login
    * `PermitRootLogin no`
  * パスワード認証の無効化
    * `PasswordAuthentication no`
  * 設定確認
    * `sshd -t`
  * sshd 再起動
    * `service ssh restart`

## カメラモジュール

### カメラハードウェア

RPi5 からケーブルが細くなった。
しかし、AI Camera には細いケーブルも同梱されているので
ケーブルを別に買う必要はない(一敗)。

ケーブルのソケットの仕様が分かりにくいが、プラスチックのパーツを差し込み方向と平行に
引くことができる。その状態だと緩んでいるのでケーブルを差し込み、再度パーツを押せば
ロックされる。

### カメラソフトウェア

* カメラ関連コマンドはさらに libcamera から rpicam に変更になった。

旧情報

* カメラモジュールの有効化
  * 従来のカメラ関連コマンドはレガシー扱いとなり非推奨となった。
  * sudo raspi-config
  * Interfacing options
  * legacy camera supprt
  * 有効にしても raspistill 等のコマンドが使えない。。
* 移行先は libcamera
  * libcamera-still が raspistill 互換 (多分)。
  * libcamera-jpeg との関係は不明。
  * legacy camera supprt = ON だと使えないっぽい。
* 静止画撮影(要 video グループ) (旧)
  * `raspistill -t 1 -o pic.jpg`
  * -t 指定すると真っ黒になってしまうことがあるらしい？
    <https://mizukama.sakura.ne.jp/blog/archives/4022>
* サイズ
  * 3280x2464
* exif のサムネイル
  * 64x48

## I2C

* I2C の有効化
  * `sudo raspi-config`
  * Interfacing Options
  * Enable I2C

* Device files
  * `ls /sys/bus/i2c/devices`

* i2c-tools
  * `sudo apt install i2c-tools`

* i2c-detect
  * `i2cdetect -l`
    * バスの列挙
    * i2c-X の X が識別子
  * `i2cdetect -F <X>`
    * 利用可能な機能
  * `i2cdetect [-y] <X>`
    * 応答のある I2C アドレスを表示
    * 警告が出る通り、変な状態になる可能性は否定できないのでその場合はリセット

## HTTP Server

* lighttpd
  * `sudo apt install lighttpd`
  * `/etc/lighttpd`
  * `lighttpd-enable-mod`, `lighttpd-disable-mod`
  * `service lighttpd force-reload`
  * 以下などを必要に応じて変更しながら enable する
    * accesslog
    * userdir
    * fastcgi
    * fastcgi-php
    * CGI の stderr
      * `server.breakagelog = "/var/log/lighttpd/breakagelog.log"`

* php
  * `apt install php-cgi`
    * これは fastcgi じゃないらしい…。
  * `apt install php-fpm`
    * FPM (FastCGI Process Manager) は、PHP における FastCGI 実装です。
      (主に)高負荷のサイトで有用な機能が含まれています。
    * Apache ではこちらが標準になったとか。
      セキュリティと性能の両面でこちらの方がいいかもしれない。
    * ただしプロセスが残るので設定ファイルを書き換えても元のが残りそう。
      `service php-fpm8.4 force-reload` 等が必要と思われる。
  * `/etc/php/.../php.ini`
  * デフォルトのアップロードサイズ制限は厳しいので適切に設定し直す。
    * memory_limit
    * post_max_size
    * upload_max_filesize
  * `service php8.4-fpm restart`
  * PukiWiki が動かない時は mb (multi byte) ライブラリ不足。
    * `apt install php-mbstring`
    * `extension=mbstring` の行をコメント解除する。

```txt
PHP Fatal error:
Uncaught Error: Call to undefined function mb_strrpos() in ...php:XXX
```

## SSL (Let's Encrypt)

* `sudo apt install certbot`
* http サーバ稼働状態でドメインと webroot (/var/www/html/ 的な位置) を
  入力するだけで自動的にドメイン証明書を作ってくれる。
  cron で自動更新設定もしてくれる。

`sudo certbot`

```text
Certbot doesn't know how to automatically configure the web server on this system. However, it can still get a certificate for you. Please run "certbot certonly" to do so. You'll need to manually configure your web server to use the resulting certificate.
```

`sudo certbot certonly`

```text
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
(lighttpd のデフォルトドキュメントルートの場合)
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

`/etc/cron.d/certbot` に cronjob が登録されている。
12 時間ごとに自動で renew してくれるらしい？

* lighttpd に設定する。
  * /etc/lighttpd/conf-available/10-ssl.conf をコピーして使う。
  * `sudo lighttpd-enable-mod (xx- と .conf を除いた名前)`
  * `sudo service lighttpd force-reload`
  * 昔は手動で結合する必要があったが、lighttpd のアップデートで必要なくなった。

```text
ssl.pemfile = "/etc/letsencrypt/live/yappy.mydns.jp/fullchain.pem"
ssl.privkey = "/etc/letsencrypt/live/yappy.mydns.jp/privkey.pem"
```

セキュリティや設定の確認は Qualys SSL LABS で診断してもらうのがおすすめらしい。
ドメインを入れるだけで色々とチェックしてくれる。

## PukiWiki

懐かしさはあるが、現代のセキュリティに対応できているかというと不安になる。
~~Wordpress よりまし。~~

<https://pukiwiki.sourceforge.io/>

海外からめちゃくちゃ荒らされる (残念だが当然) ので認証は必須。
この認証で大丈夫なのかは不明。

```php
// pukiwiki.ini.php

// Title of your Wikisite (Name this)
// Also used as RSS feed's channel name etc
$page_title = 'PukiWiki';
// Site admin's name (CHANGE THIS)
$modifier = 'anonymous';
// Site admin's Web page (CHANGE THIS)
$modifierlink = 'http://pukiwiki.example.com/';

// User definition
$auth_users = array(...);
// Edit auth (0:Disable, 1:Enable)
$edit_auth = 0;
```

ファイルアップロードに関してはおそらく `php.ini` の設定も必要。

```php
// plugin/attach.inc.php

// Max file size for upload on script of PukiWikiX_FILESIZE
define('PLUGIN_ATTACH_MAX_FILESIZE', (1024 * 1024)); // default: 1MB
```

## MySQL (MariaDB)

* `sudo apt install mariadb-server mariadb-client`
* `sudo apt install php-mysqlnd`
  * PHP (WordPress) から呼び出す場合

### セキュリティ初期設定

以下 `10.11.4-MariaDB` の情報。

`sudo mariadb-secure-installation` (`mysql_secure_installation`)

推奨やデフォルトが変わってトラブルを起こしている気がする。。

* Enter current password for root
  * sudo しておくこと。最初は設定されていないので空で OK。
* Switch to unix_socket authentication \[Y/n]
  * <https://mariadb.com/kb/en/authentication-from-mariadb-10-4/>
  * どうやら DB 側で Linux とは別の root や各ユーザの管理を行うのではなく、
    UNIX domain socket (同一マシン内接続) の際にパスワード認証をスキップすることで
    Linux ユーザの方にアカウント管理を一元化して権限管理をシンプルにしようという
    方針らしい。
  * root に関しては最初からこれがデフォルトになったという記述もあり、
    なんだかよく分からない。
    クリーンインストール直後に試してみると、Linux root からなら確かに
    パスワードなしで入れる。
  * root パスワードを空のままにされて、誰でも最大権限でデータベースの全操作が
    行われる状態のまま放置されるのを、Linux root 認証と同一化するのが主目的らしい。
  * ユーザとパスワードによる認証もパスワード強度やパスワードの置き場所問題から
    いろいろと限界が近いように感じる。かといって他の認証も作るのも正しく運用するのも
    大変で、Linux のアカウント管理+認証システムに一体化してしまうのは
    良い方向性なのかなと思う。
  * しかし結局、現在運用中のシステムが移行トラブルを起こすんだが…。
  * web server (+ web app) は www-data ユーザで動いているので、
    www-data ユーザに適切な権限を付与して動かすのがこれからはよいのかもしれない。
    ただ、テスト用のデータベースとユーザ (dev, staging) みたいなのはちょっと困るかも。
* Change the root password? \[Y/n]
  * Yes がデフォルトになっているが、上のおかげで n でも問題なくなっている。
* Remove anonymous users? \[Y/n]
  * はい。匿名ユーザは削除で。
* Disallow root login remotely? \[Y/n]
  * はい。リモートからの root ログインは禁止。
  * リモートの口は ssh を使い (ここでも Linux root login は非推奨だが)、
    sudo で Linux root になって db root ログインする。これで OK のはず。
* Remove test database and access to it? \[Y/n]
  * はい。誰でも読み書きできる設定のデータベースがあるが、消す。
  * 消す前に遊ぶか、適切なユーザと権限を設定したデータベースを作って遊ぶ。
* Reload privilege tables now? \[Y/n]
  * はい。

```sh
# root ログイン確認
sudo mysql

sudo mysql [-u USER] [-p]
```

-u は省略すると `'現在のログインユーザ'@'localhost'` が使われる。
パスワードを入力したい場合は -p を指定するが、管理が大変なのであまり使いたくないことも
多いかもしれない。

## WordPress - MariaDB

Prerequirements

* web server - lighttpd のインストール
* php, php-cgi のインストールと lighttpd からの設定
* MariaDB のインストールと root のセキュリティ(パスワードなしで入れるのだけはやめる)

apt に wordpress というのがあるが、apache に依存があり、インストールすると
そちらも同時にインストールされてしまう(一敗)。
それとちょっと古い。

以下から最新版をダウンロードする。

<https://ja.wordpress.org/support/article/how-to-install-wordpress/>

1. zip をダウンロードして展開する。そのまま web から見えるどこかに配置する。
1. index.php にアクセスしてみる。
1. 動かない→シンプルな PHP ファイルで動作を再確認。
1. 画面が出たがデータベースライブラリがないと言われる→php-mysql(nd) をインストール
  してサーバを再起動。
1. 画面が出た→データベース名とユーザ/パスワードを入力。\
  データベース名: wordpress とかにする。\
  ユーザー名: サーバの動作ユーザ (www-data) と同じにすると unix_socket 認証が効く。\
  パスワード: unix_socket 認証ならば空で OK。\
  データベースのホスト名: localhost で。\
  テーブル接頭辞: データベースの root がないけど wp を複数動かしたい人向け。
  デフォルトで。
1. wp-config.php を作ろうとするが、パーミッションエラーとなった場合は
  表示された内容を自前でコピペして作成する。
  wp-config.php は最終的には **400 (r--/---/---) 推奨**。

これでとりあえずボタンを押すとデータベース接続エラーになるので、
必要なユーザや権限をエラーが出なくなるまで作っていく。

### MariaDB ユーザの作成

まずログインエラーを直せるか確認しつつ行う。

```sql
-- パスワードなしで誰でも入れる。危険。
CREATE USER 'www-data'@'localhost';
-- パスワードを指定して作成。コマンドログに残るし色々と何とも言えないところがある。
CREATE USER 'www-data'@'localhost' IDENTIFIED BY 'password';
-- unix_socket による認証。www-data ユーザならパスワードなしでログインできる。
CREATE USER 'www-data'@'localhost' IDENTIFIED VIA unix_socket;
```

```sh
# sudo は実は root で実行する、ではなく switch user して実行する、なので
# www-data ユーザとして実行できる
# 'www-data'@'localhost' としてログイン確認
sudo -u www-data mysql
```

ここまで確認できたら WordPress からユーザ名: www-data, パスワード: 空で
データベースログインまで通ることを確認する。

### 文字コードデフォルト設定

```sql
SHOW VARIABLES LIKE 'char%';
SHOW VARIABLES LIKE "col%";
```

`/etc/mysql/my.cnf` がルート設定ファイルだが、ディレクトリの中身を全部
インクルードしているだけなので、`/conf.d/mysql.cnf` を編集する。

```text
[mysqld]
character-set-server=utf8mb4
collation-server=utf8mb4_bin

[client]
default-character-set=utf8mb4
```

```text
+--------------------------+----------------------------+
| Variable_name            | Value                      |
+--------------------------+----------------------------+
| character_set_client     | utf8mb4                    |
| character_set_connection | utf8mb4                    |
| character_set_database   | utf8mb4                    |
| character_set_filesystem | binary                     |
| character_set_results    | utf8mb4                    |
| character_set_server     | utf8mb4                    |
| character_set_system     | utf8mb3                    |
| character_sets_dir       | /usr/share/mysql/charsets/ |
+--------------------------+----------------------------+

+----------------------------------+--------------------+
| Variable_name                    | Value              |
+----------------------------------+--------------------+
| collation_connection             | utf8mb4_general_ci |
| collation_database               | utf8mb4_general_ci |
| collation_server                 | utf8mb4_general_ci |
| column_compression_threshold     | 100                |
| column_compression_zlib_level    | 6                  |
| column_compression_zlib_strategy | DEFAULT_STRATEGY   |
| column_compression_zlib_wrap     | OFF                |
+----------------------------------+--------------------+
```

character_set_system は utf8(mb3) のままで OK。

なぜか collation-server のデフォルト設定が効かない気がする。
とはいえデフォルト設定に頼るのは移行時の事故の元なので、
`CREATE DATABASE` 時に明示的に設定するようにする。

```sql
-- データベース設定情報の取得
SELECT * FROM INFORMATION_SCHEMA.SCHEMATA;
```

```text
*************************** 5. row ***************************
              CATALOG_NAME: def
               SCHEMA_NAME: wordpress
DEFAULT_CHARACTER_SET_NAME: utf8mb4
    DEFAULT_COLLATION_NAME: utf8mb4_bin
                  SQL_PATH: NULL
            SCHEMA_COMMENT:
```

### スロークエリ

WordPress 運用ではそこまで要らないかもしれないけど、一応。

```text
[mariadb]
slow_query_log
long_query_time=1.0
# default=/var/lib/mysql/{host}-slow.log
slow_query_log_file=slow.log
```

## WordPress - 本体

### パーミッションと自動更新

データベースだけでなく、WordPress のインストール先自体を書き換えることがある。
とりあえず読み取りさえできれば動きはするが、FTP の設定を求められたりする。

* 自動更新 (wp-config.php 以外のメインファイル全体)
* .htaccess
* wp-content/ 以下
  * アップロード
  * テーマ
  * プラグイン

本体の自動更新をするには PHP プログラム (= web サーバ = www-data ユーザ) から
自分自身を書き換えられるようにする必要がある。

理想

* プログラムメインファイル
  * owner = root 等、other からは read only
  * 手動で更新する
* wp-content/
  * ここだけ writable

しかし、更新作業が面倒とか、忘れているとかでセキュリティパッチの適用なしで
インターネット上に放置されるのはあまりにも危険。

現実的には

* wordpress ディレクトリ以下すべて
  * owner = www-data:www-data
  * rw-r--r--: ファイル
  * rwxr-xr-x: ディレクトリ
  * r--------: wp-config.php

こんなところかもしれない。
自分自身を任意のプログラムに置き換えられる可能性は頭にちらつくが、
セキュリティアップデートなしで放置するよりはマシな気がする。
(S)FTP or SSH を設定するというのも、もっと権限の高い権限へのログイン情報を
WordPress に持たせる感があって何とも言えない。

```sh
chown -R www-data:www-data .
find . -type f | xargs chmod 644
find . -type d | xargs chmod 755
chmod 400 wp-config.php
```

### 自動更新設定

初期設定ではなかなか激しいことになっている。。(ver 6.4.3)

```text
このサイトは WordPress の新しいバージョンごとに自動的に最新の状態に保たれます。
メンテナンスリリースとセキュリティリリースのみの自動更新に切り替えます。
```

プラグインを入れたりしていると、普通に破壊されると思われる。
そういうまともな使い方を始めるまでは常時最新アップデート設定で、
というメッセージなのかもしれないけど。。

下のリンクをクリックするとマイナー/セキュリティリリースのみの自動更新になる。

```text
このサイトは WordPress のメンテナンスリリースとセキュリティリリースのみで自動的に最新の状態に保たれます。
```

ただ、固定したバージョンも永久に使い続けられる訳ではないので、
データベースも含めたバックアップ/リストア手順と共に、
メジャーバージョンアップの手順確立と習慣化も考えるべきだと思われる。
また、PHP や MySQL (MariaDB) も数年でサポートが切れる。

一応 10 年前までのリリースにもセキュリティパッチは出ているようだけど、
10 年分の全部のリリースブランチにパッチを当ててリリース作業を行う担当者の
悲痛な叫び声が聞こえる。。

<https://ja.wordpress.org/download/releases/>

```text
積極的に保守されている6.4系統の最新版以外の以下のバージョンは、安全に使用することはできません。
```

### サイトヘルス

```text
警告 オプションのモジュール curl がインストールされていないか、無効化されています。
警告 オプションのモジュール dom がインストールされていないか、無効化されています。
警告 オプションのモジュール imagick がインストールされていないか、無効化されています。
警告 オプションのモジュール zip がインストールされていないか、無効化されています。
エラー 必須モジュール gd がインストールされていないか、無効化されています。
警告 オプションのモジュール intl がインストールされていないか、無効化されています。
```

```sh
apt install php-gd
apt install php-curl php-dom php-imagick php-zip php-intl
service lighttpd restart
```

```text
REST API で予期しない結果が発生しました
```

パーマリンク設定のところが %postname% を含むもの (デフォルトもそう) になっていると
REST API が 404 になるらしい。

```text
サイトのパーマリンク構造を選択してください。%postname% タグを含めるとリンクが理解しやすくなり、投稿が検索エンジンで上位に表示されるのに役立つ可能性があります。
```

バグのような気もするし、そのうち直るのかもしれない…。

## pCloud

買い切りのクラウドストレージ。
生涯 (会社が潰れるまで) 使い続けられるらしい。

### rclone

<https://rclone.org/>

クラウドストレージ全般に対応したコマンドラインツール。
Go 製。
暗号化した状態で保存もできるらしい。

#### インストール

<https://rclone.org/downloads/>\
<https://rclone.org/install/>

```sh
sudo -v ; curl https://rclone.org/install.sh | sudo bash
```

例によってシェルスクリプトの実行がどうかと思う場合はマニュアル操作で行う。
CPU に合わせて AMD64 (PC) or ARM64 (RasPi) を選択。
Debian/Raspbian なら *.deb パッケージを選ぶと管理が楽。
`sudo apt install ./xxx.deb` のように `./` で始まるパスを書けば
apt からインストールできる。

#### remote の追加

```sh
rclone config
```

1. New remote
1. Enter name for new remote. リモート設定に名前を付ける。
  設定完了後にコマンドライン上で毎回指定することになる。
1. Pcloud
1. Leave blank normally. と言われたら空のままで。
1. Say Y if the machine running rclone has a web browser you can use.
  多分ウェブブラウザの使えるマシン上で認証して、設定をコピーするのが楽。
  GUI 環境のある PC でここに Y と答える。
1. WSL だとブラウザが開けず `http://127.0.0.1:53682/...` へアクセスしろと
  言われるので、ブラウザで開く。
1. ブラウザから pcloud にログインしたことがあれば認証通るはず。
1. ここまで行う、または GUI 環境がない場合は `~/.config/rclone/rclone.conf` の
  内容を認証できているマシンからコピーしてくる。

OAuth とかの認証のため、ウェブブラウザがないとしんどい。
GUI のある PC にも rclone をインストールし、ウェブブラウザから認証して
アクセストークンを入手する。
設定ファイル `~/.config/rclone/rclone.conf` にアクセストークンが書かれているので、
CUI 環境での設定ファイルにコピーするのが多分楽。

#### rclone 使い方

リモートのパスは `<remote>:<path/to/file_or_dir>` のように、
設定でつけた名前をコロンの前に指定する。
パスは `/` で始めないことを推奨。
ルートを指定したい場合は空文字列で OK。
ごく一部のサービス相手では最初を `/` で始めるか始めないかで、
ルートからのパスかホームディレクトリからのパスかを使い分けられる。

```sh
# ls (--max-depth 1 をつけないと再帰的に全エントリを列挙する)
rclone ls remote:
# ls -l っぽいもの (同上)
rclone lsl remote:
# ディレクトリのみ列挙する (-R で再帰的に列挙)
rclone lsd remote:
# ファイルのみ列挙する (同上)
rclone lsf remote:
# JSON フォーマットで出力する (同上)
rclone lsjson remote:
```
