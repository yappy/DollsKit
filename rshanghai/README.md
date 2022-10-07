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
--no-deps をつけないと、全ての依存先に対してドキュメント生成されて大変なことになる。
```
$ cargo doc --no-deps
```
