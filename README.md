# DollsKit
Github Actions:
[![Rust](https://github.com/yappy/DollsKit/actions/workflows/rust.yml/badge.svg)](https://github.com/yappy/DollsKit/actions/workflows/rust.yml)
[![Rust](https://github.com/yappy/DollsKit/actions/workflows/doc.yml/badge.svg)](https://github.com/yappy/DollsKit/actions/workflows/doc.yml)

yappy家の管理プログラム

## ドキュメント
https://yappy.github.io/DollsKit/

## ソースの入手
```
$ git clone <this_repository>
```

## ビルド
環境の整った人形、または PC の中で

```
$ cd rshanghai
$ cargo build --release
# or
$ cargo b -r

# 以下は debug build となる
$ cargo build
```

Stable 版の Rust 環境があればビルドできるはず。

## 管理プログラムの実行開始
### 仮実行
```
$ cargo run --release
# or 
$ cargo r -r
```

### 設定
起動には設定ファイルが必要です。
初回起動時は設定ファイルが存在しないためエラーになりますが、
デフォルト設定ファイルが生成されるのでそれをコピーして作成してください。
ほぼすべての機能はデフォルトでは無効になっています。
存在しないキーはデフォルトファイルの内容が使われます。
```
$ cp config_default.json config.json
```

### 本実行
```
$ cargo run --release
# or 
$ cargo r -r
```

### daemon として実行
`--daemon` オプション付きで実行します。
```
$ cargo run --release -- --daemon
or
$ cargo r -r -- --daemon
```

ただし、`--daemon` なしでもよいので一度実行すると `exec.sh`, `kill.sh` が
生成されるので、そちらを実行する方が便利です。
```
$ ./exec.sh

$ ./kill.sh
```

### シグナル
* SIGINT
* SIGTERM
  * プログラムを終了します。
* SIGHUP
  * (プロセスを終了せずに) 再起動します。設定やリソースファイルのリロードに使えます。

## システム起動時に自動起動
一度実行すると `cron.txt` ができます。
```
$ crontab < cron.txt
```

## 設定ファイル (config.json) のヘルプ
ドキュメントの config_help モジュールにまとまっています。

https://yappy.github.io/DollsKit/doc/rshanghai/target/doc/rshanghai/config_help/index.html

## テストのビルドと実行
```
$ cargo test
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
