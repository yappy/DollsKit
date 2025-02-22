# Rust

## インストール

<https://www.rust-lang.org/ja/tools/install>

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Q. 最近流行ってるけどこういうの怖くない？  
A. はい。シェルスクリプトを自分で読む等、自己責任で。

## Daily Commands

### アップデートの確認

```sh
rustup check
```

確認せず直接 `rustup update` でも問題ない。

### アップデート

```sh
# rustup 自身のアップデート
rustup self update
# rust のアップデート
rustup update
cargo --version
```

### アンインストール

(やったことない)

```sh
rustup self uninstall
```

### vscode 拡張

`rust-analyzer` が公式かつつよそう。
入力補助が結構すごい。
vscode で検索して入れればよいので rustup の必要はなし。

rts (Rust Language Server) は廃止されたので非推奨。

### rustfmt

<https://github.com/rust-lang/rustfmt>

フォーマッタ。
`rustfmt.toml` で設定を変更できるが、`} else {` は unstable 扱いで
nightly ツールでないとまだ変更できない。

```sh
# 適当なインデントで書いて、これ一発で OK!
cargo fmt
# ファイルを変更するのではなく、警告を出して異常終了する
# 自動ビルドでのチェック用
cargo fmt -- --check
```

ソースコード上で `#[rustfmt::skip]` をつけると部分的に無効にできる。

### clippy (lint)

いわゆる静的解析ツール。
怪しい書き方をしている部分を指摘してくれる。

```sh
# 最近は勝手に入るようになった気がする
# rustup component add clippy
cargo clippy --no-deps
# 自動で修正する
# cargo fmt が必要な修正のされ方をされる可能性がそれなりにあるのには注意
# git 差分がある状態で行うとエラーになる
# メッセージに従ってオプションを追加すれば無理やり修正してもらえる
cargo clippy --fix
```

`--no-deps` をつけないと、全ての依存先に対してチェックが働き遅い。
ただし `cargo check` で依存先全体のチェックは行ってしまうらしいので
あまり変わらない気もする。

### doc

```sh
cargo doc --no-deps
```

`--no-deps` をつけないと、全ての依存先に対してドキュメント生成されて大変なことになる。

`--document-private-items` はバイナリクレートではデフォルトで有効になるので、
public でない要素も出力される。

`--open` をつけるとブラウザで開いてくれるそうだが、WSL では動かない。。

### update

```sh
cargo update
```

`Cargo.toml` に書かれた依存パッケージのバージョンを上げる。
`Cargo.lock` が更新されるので、それをコミットすれば OK。
github のセキュリティボットからの警告もだいたいこれで対応できる。

### cargo-edit

```sh
cargo install cargo-edit
```

`Cargo.toml` を自力で編集していたのをコマンドで自動化する。

`cargo add` は v1.62 `cargo rm` は v.1.66 から標準搭載になった。

```sh
cargo add regex
# dev-dependencies + specify version
cargo add regex@0.1.41 --dev
# build-dependencies
cargo add regex --build
```

多重に追加しても何も起こらず、また、feature list が表示される。とても便利。
`cargo add --features <FEATURES>` (カンマ or スペース区切り) で指定できる。

```sh
cargo add serde
    Updating crates.io index
      Adding serde v1.0.188 to dependencies.
             Features:
             + derive
             + serde_derive
             + std
             - alloc
             - rc
             - unstable
```

`cargo upgrade` で `Cargo.toml` 内のバージョンを上げられる。
`cargo update` は `Cargo.lock` を更新するのみ。

```sh
cargo upgrade
# --incompatible (-i) をつけると非互換アップデートを許す
# おそらくビルドエラーを起こすので使い方の修正が必要
cargo upgrade -i
```
