# Codex

ChatGPT を出している OpenAI が出しているコーディングエージェント。
でもコーディングだけでなく、設計プラニングレビュー等何でもできる、
優秀なソフトウェアエンジニア。

<https://openai.com/ja-JP/codex/>

## 料金体系

めちゃくちゃ分かりづらい、と ChatGPT 自身が認めるレベル。

OpenAI のページに行くとログインが2通りある。
そしてアカウントと課金システムが全く別のものになっている。
ただしバックエンドは同じで GPT-5.6 Luna とかそんな感じの OpenAI 製 AI。
ただし利用できるモデルには細かな違いがある。
使用中モデルが分からなかったら、本人に "あなたのモデルは何ですか" などと聞けば
教えてくれる。

* ChatGPT
  * こちらが一般向けサービス。
  * 月ごとの課金でサブスクリプション。
  * 月額課金で枠の中で使い放題。使わないと損。
    枠を使い切ったら復活するまで待つか、追加課金。
  * ごめんこっち課金してないから分かんない。
    テキストチャットだけなら無料枠でも結構使えるはず。
* API Platform
  * こちらは Web API で
  * 従量課金。入出力トークン数に応じて課金される。
    高性能モデルはトークン当たりの値段が高い。

## Codex 向け課金

ChatGPT または API Platform のどちらのアカウント=課金枠からも使用可能。
ただし利用可能なモデルには多少の違いがあるらしい。
Codex に対して直接課金するシステムは無い。
したがって、特に追加のアカウント登録は必要ない。

## Codex クライアントの種類

* ChatGPT の Codex (GUI アプリ版)
* IDE (VS Code とかのプラグイン)
* Codex CLI

Windows 版は非推奨で、WSL 推奨との噂も。

**Codex CLI** しか分からないので、以下は Codex CLI を仮定する。
ただし認証など、バックエンドは同じなので基本的に同じような感覚で使えるはず。

## Codex CLI インストール

<https://learn.chatgpt.com/docs/codex/cli>

### お手軽インストール (微妙?)

```sh
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

いつもの怖いシェルスクリプトダウンロード実行で簡単に使えるようにはなる。
が、アンインストールは手動のガバガバ設計なので、
気になる場合はパッケージマネージャに頼る方がよいかもしれない。
アンインストールは `~/.codex` と `~/.local/bin/codex` を消せば多分 OK。

なお、apt 用の `*.deb` とかそういうものはなく、npm (JS) or Homebrew (Mac) のみ対応。
Homebrew も元は Mac 用パッケージマネージャだったものの、
Linux 対応が進んでいるようなので、Homebrew だけ我慢して入れれば普通に使える。
他の関連ツールも同様の管理がなされていることが多いようなので、
郷に入っては郷に従えかもしれない。
Mac 使ってる人たちの成果物が丸ごと使えるようになるのもいいんでないかな。

案内ではアップデートは同じシェルスクリプトを実行しろとあるが、
`codex update` コマンドで簡単に同じことができる。

### Homebrew からインストール

<https://brew.sh/ja/>

```sh
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

なんか sudo のパスワード聞かれて怖いけど仕方がない。
~~だからサプライチェーン攻撃とか流行るんだ。~~

```txt
Warning: /home/linuxbrew/.linuxbrew/bin is not in your PATH.
  Instructions on how to configure your shell for Homebrew
  can be found in the 'Next steps' section below.
...
==> Next steps:
- Run these commands in your terminal to add Homebrew to your PATH:
    echo >> /home/yappy/.bashrc
    echo 'eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv bash)"' >> /home/yappy/.bashrc
    eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv bash)"
- Install Homebrew's dependencies if you have sudo access:
    sudo apt-get install build-essential
  For more information, see:
    https://docs.brew.sh/Homebrew-on-Linux
```

なんか結構重要なお知らせが出るので対応しておく。

* 言われた通りに `.bashrc` でパスを通す。
* なんか apt で build-essential が必要らしい？~~まあみんな入れてるからいいよね。~~

うまくいくと `brew` コマンドに応答するようになる。
これで codex をインストールする。
`--cask` は新しいタイプのパッケージらしい。

```sh
brew install --cask codex
```

brew 経由だと `codex update` コマンドは失敗してしまった。
`brew upgrade` を使えばよさそう。

```sh
brew upgrade --cask codex
```

### npm からインストール

npm は apt にあるが、古いのでどうなるかわかんない。

## ログイン

初回起動時、ログイン方法を聞かれる。
3番目が API Platform ログイン。

```txt
  Welcome to Codex, OpenAI's command-line coding agent

  Sign in with ChatGPT to use Codex as part of your paid plan
  or connect an API key for usage-based billing

> 1. Sign in with ChatGPT
     Usage included with Plus, Pro, Business, and Enterprise plans

  2. Sign in with Device Code
     Sign in from another device with a one-time code

  3. Provide your own API key
     Pay for what you use
```

次回からは自動でログインするが、`codex logout` で保存情報削除が可能。
ログインのみは `codex login [--<MODR>]` で可能。

**デフォルトでは値段の高い最新モデルが選択されている場合があるので注意。**
`/model` コマンドで変更できる。
