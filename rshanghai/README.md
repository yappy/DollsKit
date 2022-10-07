# Rust
## インストール
https://www.rust-lang.org/ja/tools/install

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Q. 最近流行ってるけどこういうの怖くない？  
A. はい。シェルスクリプトを自分で読む等、自己責任で。

## アップデート
```
# rustup 自身のアップデート
$ rustup self update
# rust のアップデート
$ rustup update
$ cargo --version
```

## アンインストール
(やったことない)
```
$ rustup self uninstall
```

## vscode 拡張
`rust-analyzer` が公式かつつよそう。
vscode で検索して入れればよいので rustup の必要はなし。

rts (Rust Language Server) は廃止されたので非推奨。

## clippy (lint)
いわゆる静的解析ツール。
```
$ rustup component add clippy
$ cargo clippy
```

## doc
```
$ cargo doc --no-deps
```

`--no-deps` をつけないと、全ての依存先に対してドキュメント生成されて大変なことになる。

`--document-private-items` はバイナリクレートではデフォルトで有効になるので、
public でない要素も出力される。

`--open` をつけるとブラウザで開いてくれるそうだが、WSL では動かない。。

# Tech Note
## 並列テストが失敗する
デフォルトで test attribute のついたテストは並列実行される。
ファイルやグローバル変数等のグローバル状態を変更するテストは
並列実行すると失敗する可能性がある。

`cargo test` に `--test-threads=1` をつけるとシングルスレッド実行になるが、
並列化可能なところまで直列化されてしまう。

`mod test` 内にグローバル変数として Mutex を用意し直列化したいテストで
ロックを取れば直列化できるが、assert 失敗で panic した場合に
Mutex の PoisonError で他のテストを巻き込んで失敗してしまう。

https://github.com/rust-lang/rust/issues/43155

結論としては `serial_test` クレートを使うのが便利。
```
// これがついたテストは直列化される
#[serial]
// 引数をつけるとその名前のグループ内でのみ排他される
#[serial(group)]
// serial とは排他されるが、parallel 同士は同時に実行可能
#[parallel]
// こちらもグルーピング可能
#[parallel(group)]
```
