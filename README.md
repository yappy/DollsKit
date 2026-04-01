# DollsKit

Github Actions:
[![Rust](https://github.com/yappy/DollsKit/actions/workflows/rust.yml/badge.svg)](https://github.com/yappy/DollsKit/actions/workflows/rust.yml)
[![Rust](https://github.com/yappy/DollsKit/actions/workflows/doc.yml/badge.svg)](https://github.com/yappy/DollsKit/actions/workflows/doc.yml)

yappy家の管理プログラム

## ドキュメント

GitHub Pages: <https://yappy.github.io/DollsKit/>

ドキュメントのソースは `docs/` 以下: [docs/index.md](./docs/index.md)

## ソースの入手

```sh
git clone <this_repository>
```

## ビルド

環境の整った人形、または PC の中で

```sh
cd shanghai
cargo build --release
# or
cargo b -r

# 以下は debug build となる
cargo build
```

Stable 版の Rust 環境があればビルドできるはず。

### リンクの高速化

[rust_tools.md](./docs/note/rust_tools.md) を参照。

## 管理プログラムの実行開始

### ディレクトリ

`$HOME` 相対で以下のディレクトリが使われます。
XDG 仕様に従い、`$XDG_*` 環境変数が存在すればそちらが優先して使われます。

* 設定ファイル
  * `$XDG_CONFIG_HOME` > `$HOME/.config`
* ログファイル
  * `$XDG_CACHE_HOME` > `$HOME/.cache`
* systemd サービス定義ファイル
  * `$XDG_DATA_HOME` > `$HOME/.local/share`

### 仮実行

```sh
cargo run --release
# or
cargo r -r
```

### 設定

起動には設定ファイルが必要です。
初回起動時は設定ファイルが存在しないためエラーになりますが、
デフォルト設定ファイルが生成されるのでそれをコピーして作成してください。
ほぼすべての機能はデフォルトでは無効になっています。
存在しないキーはデフォルトファイルの内容が使われます。

```sh
cd ~/.config/shanghai
cp config_default.toml config.toml
```

### フォント

※Twitter bot 向けの機能なので現在は有効にしても動作しません。

デフォルト設定で指定されているフォントファイルは以下でインストールできます。

```sh
sudo apt install fonts-ipafont
```

### 本実行

```sh
cargo run --release
# or
cargo r -r

# コンソールへの出力を詳しくする
cargo run --release -- -v
```

### シグナル

* SIGINT
* SIGTERM
  * プログラムを終了します。
* SIGHUP
  * (プロセスを終了せずに) 再起動します。設定やリソースファイルのリロードに使えます。
* SIGUSR1
  * ログをフラッシュします。
    ディスク (SD Card) の寿命対策のため、これを行わないと最新のログがファイルに
    反映されません。

## システム起動時に自動起動

一度実行するとデータディレクトリ `$HOME/.local/share` に systemd 用の
ファイルができます。
中に書かれているコメントに従って `/etc/systemd/system/` 内にシンボリックリンクを
作成し、リロードすると `shanghai.service` が利用できるようになります。

```sh
cd /etc/systemd/system
ln -s path/to/shanghai.service
systemctl daemon-reload
```

```sh
# systemd 管理下のサービスとして起動と終了
systemctl start shanghai
systemctl status shanghai
systemctl stop shanghai
# ログ (stdout/stderr) の確認
journalctl -u shanghai
```

systemd サービスとして使用可能になったら、enable コマンドで自動起動を有効化できます。

```sh
systemctl enable shanghai
```

## 設定ファイル (config.toml) のヘルプ

ドキュメントの sys::config::Config にあります。

<https://yappy.github.io/DollsKit/doc/sys/config/struct.Config.html>

## テストのビルドと実行

```sh
cargo test
```

## ドキュメントのビルド

```sh
cargo doc --no-deps --document-private-items
```

## CI

GitHub Actions で自動ビルドを行っています。
`.github/workflows/` 以下を参照。

### 自動ビルド

push および pull request 時に debug/release ビルドおよびテストを行います。

### ドキュメントの自動更新

`doc` ブランチの `docs/` ディレクトリが GitHub Pages で公開されています。
`main` ブランチに変更が push されると自動で `doc` ブランチを更新します。

1. `doc` ブランチをチェックアウト
1. `doc` ブランチに `main` ブランチをマージ
1. `cargo doc` で Rust ドキュメントを自動生成
1. `docs/` 以下を更新して commit
1. push
