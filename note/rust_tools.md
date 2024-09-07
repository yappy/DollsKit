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
```

## Tech Note

### 並列テストが失敗する

デフォルトで test attribute のついたテストは並列実行される。
ファイルやグローバル変数等のグローバル状態を変更するテストは
並列実行すると失敗する可能性がある。

`cargo test` に `--test-threads=1` をつけるとシングルスレッド実行になるが、
並列化可能なところまで直列化されてしまう。

`mod test` 内にグローバル変数として Mutex を用意し直列化したいテストで
ロックを取れば直列化できるが、assert 失敗で panic した場合に
Mutex の PoisonError で他のテストを巻き込んで失敗してしまう。

<https://github.com/rust-lang/rust/issues/43155>

結論としては `serial_test` クレートを使うのが便利。

```rust
// これがついたテストは直列化される
#[serial]
// 引数をつけるとその名前のグループ内でのみ排他される
#[serial(group)]
// serial とは排他されるが、parallel 同士は同時に実行可能
#[parallel]
// こちらもグルーピング可能
#[parallel(group)]
```

## グローバル変数

## エラーハンドリング

関数の返り値を Result にするとき、`Err<T>` 時の型が合っていないと
? 演算子で楽ができない。
このままだと関数内でいろいろな種類のエラーが返ってくる場合に困る。
そこで、標準ライブラリのエラーは `std::error::Error` トレイトを実装しているので
この型でエラーを返すようにする。

The Book でも実は
<https://doc.rust-jp.rs/book-ja/ch12-03-improving-error-handling-and-modularity.html>
で導入している。
(Result のエラーハンドリングの章にはない。。)

Rust By Example ではここ。
<https://doc.rust-jp.rs/rust-by-example-ja/error/multiple_error_types/boxing_errors.html>

オススメ

```rust
Result<(), Box<dyn Error + Send + Sync + 'static>>
```

コンパイル時に型を決定できない方法になるので Box によるヒープポインタ参照を使う。

Send, Sync 制約をつけるとスレッド間でエラーオブジェクトを受け渡せるようになる。
別スレッドで実行した結果を Result として受け取れるようになる。

'static をつけると `is` や `downcast_ref` 等が使えるようになる。

<https://doc.rust-lang.org/stable/std/error/trait.Error.html#method.is>

## エラーハンドリング - anyhow

標準ライブラリのみで行うならば前節の通り。

<https://qiita.com/legokichi/items/d4819f7d464c0d2ce2b8>
スマートなエラー処理に関しては昔からかなり苦労しているらしい…。

anyhow はこの辺りをもっと簡単に書けるようにしてくれるライブラリ。
2019 年頃登場し、2022 年現在デファクトと言える。
特に標準ライブラリのみで頑張る必要がないのであれば、ほぼすべてのプロジェクトの開始時に
依存を張ってよいタイプのライブラリと言えそう。
(個人的にはあまりそういうのは好きではないけど、Rust は基本的なところですら変遷期であり、
しばらく時間が経てば標準入りするんじゃないかな？)

<https://docs.rs/anyhow/latest/anyhow/>

使い方は、とりあえず全部 `anyhow::Result<T>` を返せば OK!
これは `Result<T, anyhow::Error>` に等しい。
これで `std::error::Error` を返せるようになるので、普通のエラーなら何でも
? 演算子で返せるようになる。
(`From` trait によって実現されているっぽい。)

`std::process::Termination` を実装しているので main から返しても大丈夫。

```rust
fn main() -> anyhow::Result<()> {
    Ok(())
}
```

Send + Sync 問題とかも解決しているので async fn から返しても大丈夫。

ダウンキャストの確認や実行も可能。

`context()` を使うと情報を付け加えつつエラーを投げられる。

```rust
fn main() -> Result<()> {
    ...
    it.detach().context("Failed to detach the important thing")?;

    let content = std::fs::read(path)
        .with_context(|| format!("Failed to read instrs from {}", path))?;
    ...
}
```

nightly channel を使うか features "backtrace" を指定するとバックトレース機能が
使えるようになる。(そのうち安定化して標準化するかも？)

`anyhow::Error` を作りたい場合には `anyhow!` マクロが便利。
`format!` と同じ用法でエラーを作れる。

`bail!` は `return Err(anyhow!(...))` でより簡単に early return できる。
`ensure!` は `ensure!(user == 0, "only user 0 is allowed");` のように
if まで省略して early return できる。
