# GROWI docker definition

## git submodule

GROWI 公式の docker compose テンプレート growi-docker-compose を
submodule として登録しています。
git clone, pull しただけでは更新されません。

```sh
# 1回でよい
git submodule init
# 更新があったら毎回
git submodule update

# 同時にやる
git submodule update --init
# submodule の中の submodule も含める
# 何も考えたくない人向け
git submodule update --init --recursive

# clone 時に一緒にやってしまう
git clone --recursive
```

なおあまりに面倒な上に update を忘れると壊れるので以下の設定がおすすめ。
ただし最初の一回は避けられないかも？

```sh
git config --global submodule.recurse true
```

## systemd

サービス化、自動起動について

### サービスの追加

`/etc/systemd/system/` に `*.service` という名前で設定ファイルを置く。
シンボリックリンクも既に置かれていたので多分大丈夫。
編集後はリロードが必要。

下記の `enable` `disable` で自動起動を設定できる。

### systemctl

`systemctl <CMD>`

* `daemon-reload`
  * 設定ファイルを編集した後にリロードする。
* `start <SERVICE>`
* `stop <SERVICE>`
  * 起動と停止。
* `is-enabled <SERVICE>`
* `enable <SERVICE>`
* `disable <SERVICE>`
  * 自動起動の設定。

### journalctl

覚えづらそうな名前をしているが、失敗時に stdio が見えないと詰むので覚えておく。

`journalctl -ex`

* `-e`
  * ページャーを最後に送った状態で開始。
* `-x`
  * 詳細なログを表示。
  * これをつけないと stdout, stderr が見えない。**失敗時は必須。**
* `-u <SERVICE or etc.>`
  * 特定のユニットのログに絞る。
* `-f`
  * 更新を表示し続ける。(`tail -f` みたいなの)

## バックアップ

### ボリューム

コンテナの中は隔離されたファイルシステムで、コンテナを破棄 (compose down)
すると消えてしまう。
ボリュームはこれに対する永続化用の領域で、概念は外付けディスクに近い。
`docker volume <CMD>` で個別に操作可能だが、compose の定義から作られることも多い。

```txt
Commands:
  create      Create a volume
  inspect     Display detailed information on one or more volumes
  ls          List volumes
  prune       Remove unused local volumes
  rm          Remove one or more volumes
```

growi だとこんな感じで作って

```yaml
volumes:
  growi_data:
  mongo_configdb:
  mongo_db:
  es_data:
  page_bulk_export_tmp:
```

こんな感じでボリューム名とマウント先を指定してコンテナから使われている。

```yaml
volumes:
  - mongo_configdb:/data/configdb
  - mongo_db:/data/db
```

実際には最初にプロジェクト名を付けて他のプロジェクトと名前空間を分けている。

```sh
$ docker volume ls
DRIVER    VOLUME NAME
local     growi-public_es_data
local     growi-public_growi_data
local     growi-public_mongo_configdb
local     growi-public_mongo_db
local     growi-public_page_bulk_export_tmp
```

### バインドマウント

ボリュームと同じオプションを使うため非常に分かりづらいが、
ホストのファイルシステムをマウントしてコンテナ内外でファイルを共有することができる。
ボリュームとは割と別の概念。

名前は Linux の `mount --bind` から来ていると思われる。
ファイルツリー中のあるパスを、別のディレクトリにマウントし、2種類以上のパスから
同一の内容が見えるような仕組みである。BSD 系の nullfs (理解者理解)。
中身は unionfs を使わずホストのファイルシステムを bind mount して
そのまま見せているだけらしい。~~VM と違って楽でいいね。~~

```sh
docker run --volume <host-path>:<container-path>
```

ホストパスが無いと自動的にディレクトリが作られるが、終了時に自動削除はされない。
されたら困るけど。

### busybox

ボリュームはコンテナの生成破棄とは別に管理されるので、他のコンテナにマウントして
調査やバックアップを行うことができる。

DockerHub にある busybox は最低限のいつものコマンドだけを集めた軽量がウリのイメージ。

```sh
# コマンドを実行して終了
# --rm をつけないと終了したコンテナが docker container ls -a に残る
docker run --rm busybox ls
# -i -t で stdin をコンソールに接続するとコマンド入力待ちになる
docker run --rm -it busybox
# docker volume ls で得たボリューム名と、コンテナ内でのマウント先を指定
# いつものコマンドで調べる
docker run --rm -it -v growi-public_mongo_db:/tmp/db busybox
# ホストディレクトリのバインドマウントと併用
# コンテナ削除後も成果をホストファイルシステムに残せる
docker run --rm -it -v growi-public_mongo_db:/tmp/db -v ./share:/tmp/share busybox
```

ボリュームマウントとホスト fs のバインドマウントを組み合わせて起動し、
busybox のコマンドを使えば大体何でもできるはず。

### alpine

Alpine は軽量 Linux ディストリビューション。

* コマンドは BusyBox のみ
* libc が musl (glibc ではなく軽いやつ)
* パッケージマネージャ apk つき

```txt
               DISK USAGE   CONTENT SIZE
alpine:latest      13.6MB         4.28MB
busybox:latest     6.21MB         1.91MB
debian:latest       209MB         52.8MB
```

### Debian/Ubuntu

うまくできないなら諦めて Ubuntu + apt でも使うといいよ(適当)。
バージョンによりサイズが変動しているので歴代サイズまとめサイトを見て選択する。
そもそもプロダクションではバージョン固定すべき。
