# Rust

## Daily Commands

### インストール

<https://www.rust-lang.org/ja/tools/install>

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Q. 最近流行ってるけどこういうの怖くない？  
A. はい。シェルスクリプトを自分で読む等、自己責任で。

### アップデートの確認

```sh
rustup check
```

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
rustfmt.toml で設定を変更できるが、`} else {` は unstable 扱いで
nightly ツールでないとまだ変更できない。

```sh
cargo fmt
```

ソースコード上で `#[rustfmt::skip]` をつけると部分的に無効にできる。

### clippy (lint)

いわゆる静的解析ツール。

```sh
rustup component add clippy
cargo clippy --no-deps
```

`--no-deps` をつけないと、全ての依存先に対してチェックが働き遅い。

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

## Twitter API v2

Last update: 2022/10

<https://developer.twitter.com/en/docs>

"en" を "ja" に直すと日本語のページが表示されるが、
トップページ以外は英語しかないようなので諦めて英語を読む。

これの curl あたりを読めばだいたい把握できる。

<https://developer.twitter.com/en/docs/tools-and-libraries/using-postman>

<https://documenter.getpostman.com/view/9956214/T1LMiT5U>

### 登録

Developer Portal へ bot 用のアカウントでサインイン。
最初にいくつかの質問に答え、規約に同意する。

v1.1 時代に登録したアプリは Standalone Apps として残っている。
v2 ではプロジェクトという単位配下にあるものからしか v2 API のほとんどが利用できない。
プロジェクトを新規作成し、その配下に既存アプリを登録することで
v1.1 アクセスを維持しつつ v2 対応されるようだ。
Development, Staging, Production の選択ができるが、違いは謎。

今のところ Essential, Elevated, Academic Research の3つのプランがある。
Essential は特に何もしなくてもそのまま無料で使える。
Elevated は使用目的等を頑張って英作文すると無料で使える。
Academic Research も無料で、学術研究用。

プロジェクトとその配下にアプリを作成したら、アプリの設定
authentication settings で認証トークンを作成する。

### エンドポイントの例

v2 では User ID が必要。

* Standard v1.1 endpoints:
  * <https://api.twitter.com/1.1/statuses/home_timeline>
  * <https://api.twitter.com/1.1/statuses/user_timeline>
  * <https://api.twitter.com/1.1/statuses/mention_timeline>
* Twitter API v2 endpoint:
  * <https://api.twitter.com/2/users/:id/timelines/reverse_chronological>
  * <https://api.twitter.com/2/users/:id/tweets>
  * <https://api.twitter.com/2/users/:id/mentions>

### 認証

<https://developer.twitter.com/en/docs/authentication/overview>

適当なエンドポイントに認証なしでアクセスすると 401 で失敗する。

```sh
$ curl https://api.twitter.com/2/users/by/username/yappy_y
{
  "title": "Unauthorized",
  "type": "about:blank",
  "status": 401,
  "detail": "Unauthorized"
}
```

#### App Only (OAuth 2.0 Bearer Token)

<https://developer.twitter.com/en/docs/authentication/oauth-2-0>

誰でも見える public なデータは Developers Portal で生成できる Bearer Token による
アプリの認証のみで取得できるようになる。

Developers Portal のアプリのページを開き、"Keys and tokens" のタブで
"Bearer Token" を生成できる。
Revoke および再生成も可能。

HTTP header に Authorization: Bearer \<TOKEN\> を追加するだけで OK。
curl なら --header/-H オプション。

```sh
curl "https://api.twitter.com/2/tweets?ids=1261326399320715264,1278347468690915330" \
  -H "Authorization: Bearer AAAAAAAAAAAAAAAAAAAAAFnz2wAAAAAAxTmQbp%2BIHDtAhTBbyNJon%2BA72K4%3DeIaigY0QBrv6Rp8KZQQLOTpo9ubw5Jt?WRE8avbi"
```

#### OAuth 1.0a User Context

<https://developer.twitter.com/en/docs/authentication/oauth-1-0a>

昔からあるいつものやつ。
そのアカウントでしか行えない操作を行える。

Twitter の解説ページには書かれていないが、
OAuth 1.0a 自体の仕様で、request body の Content-Type が
application/x-www-form-urlencoded の場合のみ署名計算の対象になる。
つまり、Twitter API v2 では POST body は application/json であるため、
この部分は署名計算の対象にしない。

それでいいのか？という気もするが、
OAuth 1.0a `3.4.1. Signature Base String` にしっかり注意が書いてある。
(AOuth の署名だけでは request body 部分の改竄検知はできない)
