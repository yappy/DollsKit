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
