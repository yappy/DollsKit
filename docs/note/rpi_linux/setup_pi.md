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

## 初期設定 (cloud-init)

最近はブートイメージの作成時点で設定可能な項目が増えたが、これはどうやら
`cloud-init` というソフトを使っているらしい。
元々は AWS EC2 のインスタンスを簡単に初期設定するために開発されたらしい。
クラウドなんて使ってないのに、と混乱するが、確かに RasPi の初期設定にも便利である。

設定ファイルは `/boot/firmware/user-data` にある。
`/etc/cloud/` にあるのはニセモノなので注意！
ブートパーティションにテキストの設定ファイルとして置かれている。
幸い root なら書き換え可能。

ブートイメージ作成時に間違えた or 後から変えたくなった場合、このファイルを編集する。
さもないと再起動時に毎回このファイルの内容が適用され、
他の手段による設定変更が上書きされてしまう。
例えば、ホストネームを修正したい場合は `raspi-config` でも `hostnamectl` でもダメで、
このファイルを編集しなければならない(一敗)。

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

### DNS の設定追加

家の DNS の様子が怪しい場合。

`/etc/resolv.conf` は NetworkManager によって管理されており、
書き換えても再起動時に上書きされてしまう。
NetworkManager の管理ツールで設定を追加する必要がある。

```sh
# 名前を取得
nmcli device
nmcli connection modify [CONN] +ipv4.dns 8.8.8.8

systemctl restart NetworkManager
cat /etc/resolv.conf
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

## ファイアウォール

<https://www.raspberrypi.com/documentation/security/ufw.html>

```sh
echo "$SSH_CONNECTION"
```

で現在 ssh で使っているポートを確認できる。

デフォルトは拒否、22 (ssh default) のみ許可するなら以下のようになる。
ポートは必要に応じて変える・増やす。

```sh
apt install ufw
ufw default deny incoming
ufw allow 22/tcp
# well-known port なら名前でも指定可
# ufw allow ssh
ufw status
```

enable にすると ssh が締め出される危険があるため、systemd のタイマー機能がおすすめ。

```sh
# 5分後に ufw disable を実行する
sudo systemd-run --unit=ufw-rollback --on-active=5m /usr/sbin/ufw disable
sudo ufw enable
sudo ufw status verbose
```

**今のSSH接続は閉じずに、別の端末から新しくSSH接続できることを確認** する。
成功したら自動解除を取り消す。

```sh
sudo systemctl stop ufw-rollback.timer
```

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

`sudo apt install lighttpd`

[lighttpd.md](./lighttpd.md)

## Docker

<https://docs.docker.com/engine/install/debian/>

なんかデフォルトの apt にあるやつは unofficial 呼ばわりされている。
この辺は全部消さないと競合して壊れるらしい。
`apt show` で Maintainer や APT-Sources を見て Debian のものは無視して
Docker 公式のもののみを入れるよう気を付ける。
~~非互換な変更入れすぎでは。~~

* docker.io
* docker-compose
* docker-doc
* podman-docker

信用する keyrings に公式の鍵を追加して、apt source を追加する。

```sh
apt install docker-ce docker-ce-cli containerd.io \
  docker-buildx-plugin docker-compose-plugin
```

```sh
systemctl status docker
systemctl start docker
```

`docker` というグループができるので、それに追加したユーザが使えるようになるらしい。

```sh
docker run hello-world
```

`docker-compose` はプラグイン化して docker のサブコマンドになったらしい？

```sh
docker compose version
```

### Docker Daemon の設定

本体からのヘルプは `dockerd --help`。
設定ファイルの場所は `/etc/docker/daemon.json`。

Docker の保存する色々なデータは `/var/lib/docker` に置かれる。
これは `data-root` オプションで変更可能。

Docker Engine 29.0 以降をクリーンインストールした場合はイメージや
コンテナスナップショットは `/var/lib/containerd` に置かれる。
ボリュームや設定等の他のデータは `/var/lib/docker` のまま。
overlay2 のような昔のストレージドライバ (アップグレードインストールのデフォルト)
の場合はすべて `/var/lib/docker` に置かれる。

unionfs により `du` すると本来より大幅に大量のディスクを食っているように見えるが
そんなことはない。
しかしユーザランドにそのように見せているということは rsync 等のバックアップコピーが
いい感じに動くとは到底言い難い。
というか `Dockerfile` や `compose.yml` から作り直せるデータであり、
バックアップの必要はない。
バックアップ対象の /var ではない場所に設定を変えるのが無難と思われる。
Volume は別途バックアップが必要。

```json
// /etc/docker/daemon.json
{
  "data-root": "/mnt/docker/data"
}
```

```ini
# /etc/containerd/config.toml
# 以下をコメント解除して書き換え
# root = "/var/lib/containerd"
root = "/mnt/docker/containerd"
```

```sh
service containerd restart
service docker restart
```

### GROWI

TODO

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
