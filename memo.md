# 準備
- raspbian の img を落とす
- Win32DiskImager を (zipで) 落とす
- microSD に焼く
- メモカを本体に入れてから電源をつなぐ
  - あと有線 LAN も
- マウスと HDMI をつないでネットワーク設定を見るか、なんかほかの方法で IP addr を特定する
- `ssh <IP addr>`
  - user, pass = pi, raspberry
  - インターネット公開前にはポートを変えておいたほうがよい


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
