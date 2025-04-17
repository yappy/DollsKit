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

### install --list

`cargo install` で新たなコマンドをインストールできるが、一覧が見たくなった時に。

```sh
$ cargo install --list
cargo-cache v0.8.3:
    cargo-cache
cargo-edit v0.13.2:
    cargo-add
    cargo-rm
    cargo-set-version
    cargo-upgrade
cargo-expand v1.0.106:
    cargo-expand
cargo-update v16.3.0:
    cargo-install-update
    cargo-install-update-config
mdbook v0.4.48:
    mdbook
mdbook-mermaid v0.15.0:
    mdbook-mermaid
```

## install-update

`cargo install` したものはバージョンアップしても自動で更新はされない。
全部一括でやってくれるコマンド。

```sh
cargo install install-update
```

```sh
# -a = --all
cargo install-update -a
```

### cargo-edit

```sh
cargo install cargo-edit
```

`Cargo.toml` を自力で編集していたのをコマンドで自動化する。
`cargo add` は v1.62、`cargo rm` は v.1.66 から標準搭載になった。

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

## cargo-cache

そのうち気づくことになると思われるが、`$HOME/.cargo` 以下にビルド中にダウンロードした
依存クレートのダウンロードキャッシュが溜まっていき、そのうちすごいサイズになっている。
実は現状では単調増加で削除されることはないらしい…。

公式でも古くなったキャッシュを消す `cargo clean gc` コマンドを追加検討中らしい。
unstable なので stable ではまだ使えない。

<https://blog.rust-lang.org/2023/12/11/cargo-cache-cleaning.html>

```sh
cargo install cargo-cache
```

cargo-cache はこのキャッシュをいい感じに消したり圧縮したりするコマンド。

```sh
# .cargo/ のサマリを表示
cargo cache
# だいたいこれで消える
# --autoclean = -a
cargo cache --autoclean
# 他のコマンドが気になる場合
cargo cache --help
```

## リンカを mold に変更

(Rust に限った話ではないが、) 実は従来のリンカ (GNU ld, LLVM lld) は
作られた歴史が古く、マルチコアの時代に適応できておらず、
マルチスレッドで性能が出るような設計になっていない。
基本的な場所すぎてバグらせた場合のリスクが大きすぎ、そのままになっている。

コンパイルはコンパイラのプロセスを1ファイルごとに立ち上げ、同時に実行することで
並列化の恩恵を大きく受けられる。
コア数の多い人道的な CPU を使えばみるみるコンパイル時間は短縮されていく。

しかしながらリンカは全てのコンパイルが完了した後にその結果のオブジェクトファイル
全てを1つにまとめる作業であり、プロセスを分けることはできず、
ここで内部的にマルチスレッド並列化が効かないと大幅に時間を食うことになる。
そして現代のソフトウェア肥大化の影響をもろに受ける箇所である。。

というわけで、凄腕の日本人が開発した並列化対応リンカが mold である。
各環境でデフォルトになっていないのは、カーネルやシステムソフトウェアの
リンカを置き換えるにはリスクが高すぎて実績 (時間) が足りていないからである。
コンパイラではなくリンカで、Rust 専用というわけではない。
Rust はコンパイラは rustc だが、そこから先のリンカ含む binutils は
C/C++ 用のものを流用している。

参考程度だが、経緯はここにある。\
<https://note.com/ruiu/n/ndfcda9adb748>

### mold のインストール

なんかよくわかんないけど apt にあった。

```sh
$ sudo apt install mold
$ mold -v
mold 1.10.1 (compatible with GNU ld)
```

新しいのがいいならソースが GitHub にあるので自分でビルドする。
`How to Build` を参照。
ビルド難度は不明。\
<https://github.com/rui314/mold>

### mold を Rust から使う

使い方も GitHub の README に詳しい。\
<https://github.com/rui314/mold>

設定ファイルはプロジェクトの `.cargo/config.toml`、
または全てに適用したい場合は `~/.cargo/config.toml` に置く。
`.cargo/config` は古い名前なので置くと警告が出る。

以下は例だが、`linker = "clang"` を指定しているのは古い gcc が `-fuse-ld` を
受け付けてくれないからだそうなので、ダメだったら追加するで OK。
clang に変えると clang がインストールされていない場合にエラーになる (当たり前)。

```toml
[target.x86_64-unknown-linux-gnu]
# linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

target の後に target triple (クロスコンパイラとかでよく見るやつ) が必要だが、
いざ自分の環境の triple を出せと言われると出し方が分からなくてきれそうになる。
通例、triple 省略で自環境を指すため。

答えとしては `rustup show` がお手軽っぽい。

```sh
$ rustup show
Default host: x86_64-unknown-linux-gnu
```

RasPi4 だとこう。

```sh
$ rustup show
Default host: aarch64-unknown-linux-gnu
```

### リンク結果の確認

elf の `.comment` セクションにコンパイラ、リンカ等、
使用ツールのバージョンが入っている。
readelf の `--string-dump` オプションが便利だそうだが、覚えとらんわそんなん。
ビルド時間を見てあからさまに速くなっていたらそれをもって確認としてもいいのかもしれない。
`strings some.elf | grep mold` とかで十分という説もある。

```sh
$ readelf --help
  -p --string-dump=<number|name>
                         Dump the contents of section <number|name> as strings
```

```sh
$ readelf -p .comment rshanghai

String dump of section '.comment':
  [     0]  mold 1.10.1 (compatible with GNU ld)
  [    25]  GCC: (crosstool-NG UNKNOWN) 13.2.0
  [    48]  GCC: (Debian 12.2.0-14) 12.2.0
  [    68]  rustc version 1.85.0 (4d91de4e4 2025-02-17)
```

### ビルド時間の計測

`--release` とかは各自で。
`test` とかもいけるらしい。

```sh
cargo build --timings
```
