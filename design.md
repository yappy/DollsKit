# DollsKit リポジトリ概要

このリポジトリは、家サーバ / Raspberry Pi を運用するための個人用管理システム一式です。README では「yappy家の管理プログラム」と説明されています。

中心は Rust 製の常駐プログラム `shanghai` で、それにバックアップ、GROWI、cron、DB、証明書、運用ドキュメントが同梱されています。

## 主要構成

- `shanghai/`
  - Rust ワークスペース。
  - 約 1.4 万行の Rust / TOML で構成されています。
  - 家サーバ上で動く常駐管理プログラム本体です。
- `root/`
  - 実運用用スクリプト群。
  - フルバックアップ、GROWI の MongoDB バックアップ / 復元、cron、Docker、CA、DB SQL などを含みます。
- `docs/`
  - GitHub Pages 向けの運用メモ。
  - Raspberry Pi、Lighttpd、Docker、systemd、ストレージ、Windows リモート、Rust、OpenAI / Twitter API などのノートがあります。
- `.github/workflows/`
  - CI / ドキュメント生成 / HTTPS 監視用の GitHub Actions 定義があります。
- `.gitmodules`
  - サブモジュールとして GROWI 公式 docker-compose と、別リポジトリのバックアップツール `bkup` を取り込んでいます。

## Rust ワークスペース

`shanghai/` は以下のクレートで構成されています。

- `shanghai`
  - 実行バイナリ。
  - ログ初期化、systemd ファイル生成、設定ロード、タスクサーバ起動を担当します。
- `sys`
  - 常駐管理機能の本体。
  - Health、Camera、HTTP、Discord、LINE、Twitter、OpenAI などのシステムモジュールを持ちます。
- `utils`
  - 天気データ処理、HTTP 補助、画像、ゲーム / ダイス、パーサなどの共通処理。
- `customlog`
  - ファイルローテーション付き独自ロガー。
- `verinfo`
  - Git 情報やビルド情報を埋め込む補助クレート。

## 常駐プログラムの起動と設定

入口は `shanghai/shanghai/src/main.rs` です。

起動時には以下を行います。

1. コマンドライン引数を処理します。
1. stdout とファイル向けのログを初期化します。
1. systemd 用の `shanghai.service` をユーザーのデータディレクトリに生成します。
1. `~/.config/shanghai/config.toml` を読み込みます。
1. `SystemModules` を初期化します。
1. `TaskServer` で非同期タスク群を起動します。

設定ファイルは XDG 仕様に沿って `XDG_CONFIG_HOME` または `~/.config` 配下に置かれます。初回起動時には `config_default.toml` が生成され、それを `config.toml` にコピーして使う設計です。

シグナル処理は以下の通りです。

- `SIGINT` / `SIGTERM`
  - 終了。
- `SIGHUP`
  - プロセスを終了せず再起動。
  - 設定やリソースの再ロード用途。
- `SIGUSR1`
  - ログをフラッシュ。

## システムモジュール

`sys` クレートの `SystemModules` が、各機能モジュールを Tokio の非同期タスクとして管理します。

主なモジュールは以下です。

- Health
  - CPU、メモリ、ディスク、温度を定期測定します。
  - 測定履歴を保持し、通知にも使えます。
- Camera
  - Raspberry Pi カメラの定期撮影を扱います。
  - サムネイル生成、履歴管理、アーカイブ管理、容量制限による古い画像の削除を行います。
  - Raspberry Pi 以外でデバッグするための fake camera 設定もあります。
- HTTP
  - Actix Web ベースのバックエンドサーバ。
  - Lighttpd からのリバースプロキシで使う想定です。
  - アップロード、GitHub hook、LINE webhook、一時データ配信、カメラ管理画面などを提供します。
- Discord
  - Discord bot 機能。
  - 通知、コマンド、ダイス / コイン、カメラ操作、OpenAI 連携、画像生成、音声生成などを扱います。
- LINE
  - LINE webhook / bot 機能。
  - OpenAI 連携、会話履歴、画像入力バッファ、特権ユーザー判定などがあります。
- Twitter
  - Twitter API 連携。
  - タイムラインチェック、ツイート、OpenAI 連携用ハッシュタグなどの設定があります。
- OpenAI
  - OpenAI API wrapper。
  - Responses API、画像生成、音声生成、モデル情報、レート制限情報、function calling 風の内部関数群を扱います。
- SystemInfo
  - システム情報を返す軽量モジュールです。

## タスクサーバ

`sys/src/taskserver.rs` は Tokio runtime とキャンセル通知を持つタスク管理層です。

機能は大きく分けて以下です。

- one-shot task の起動。
- 分単位の時刻リストに従う periodic task の起動。
- `SIGINT`、`SIGTERM`、`SIGHUP`、`SIGUSR1`、`SIGUSR2` の監視。
- 終了時のキャンセル通知。
- 全タスク完了待ち。

各 `SystemModule` は `on_start` で必要なタスクを登録します。

## HTTP 機能

HTTP サーバは `127.0.0.1` に bind し、外側の Lighttpd などからリバースプロキシされる前提です。

代表的なエンドポイントは以下です。

- `/`
  - ルート確認用。
- `/upload/`
  - ファイルアップロード。
- `/github/`
  - GitHub webhook。
- `/line/`
  - LINE webhook。
- `/tmp/{id}`
  - 一時データ配信。
- `/priv/camera/`
  - 管理者向けカメラ画面。
- `/priv/camera/history`
  - 撮影履歴。
- `/priv/camera/archive`
  - アーカイブ。
- `/priv/camera/take`
  - 手動撮影。

## バックアップ

`root/backup/` には実運用向けのバックアップスクリプトがあります。

- `bkup_full.py`
  - `/mnt/bkup` を前提にフルバックアップを行います。
  - 対象ディレクトリの同期、アーカイブ、古いバックアップ削除、rclone によるクラウド転送を実行します。
- `bkup_growi.py`
  - GROWI の MongoDB データをバックアップします。
  - 通常の GROWI service を止め、メンテナンス用 service で MongoDB のみ起動し、`mongodump` を取得します。
  - 取得後にアーカイブ化し、必要に応じてクラウド転送します。
- `restore_growi.py`
  - `mongorestore` による GROWI データ復元用スクリプトです。
- `bkup/`
  - サブモジュール。
  - rsync / robocopy / tar / 7-Zip / rclone などを使う汎用バックアップツールです。

cron 用には `root/cron/bkup` と `root/cron/growi_bkup` があり、ログを `/root/log/` 配下に出力する想定です。

## GROWI

`root/growi/` には GROWI の Docker Compose と systemd service 定義があります。

- `compose.yaml`
  - GROWI 公式 docker-compose へのローカル上書き設定。
  - `APP_SITE_URL`、ファイルアップロード方式、MathJax、Elasticsearch メモリ設定などを指定しています。
- `growi.service`
  - GROWI 通常起動用の systemd service。
- `growi-maintenance.service`
  - MongoDB のみを起動するメンテナンス用 systemd service。
  - バックアップ時に使われます。

## DB / CA / その他運用ファイル

- `root/db/`
  - MariaDB / MySQL 向けと思われる SQL があります。
  - WordPress 用 DB / ユーザー作成 SQL も含まれます。
- `root/ca/`
  - OpenSSL 設定や CA 作成用スクリプト / メモがあります。
- `root/docker/`
  - Docker 操作用の小さな補助スクリプトがあります。
- `misc/fio/`
  - ストレージ性能測定用と思われる fio 設定があります。

## CI / ドキュメント生成

GitHub Actions では以下が設定されています。

- Rust
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features`
  - `cargo doc`
  - `cargo build`
  - `cargo test`
  - release build
- Python
  - flake8。
- Shell
  - shellcheck。
- ドキュメント
  - `main` ブランチへの push 時に `doc` ブランチを更新。
  - Rust doc を `docs/doc/` に生成して GitHub Pages に公開する流れです。
- HTTPS 監視
  - スケジュール実行で `https://yappy.mydns.jp/` に `curl -f` する workflow があります。

## 全体像

DollsKit は、汎用ライブラリや単体アプリというより、個人宅サーバの「実アプリ + インフラ運用手順 + バックアップ / 復旧スクリプト」をまとめた管理リポジトリです。

Rust 側は常駐管理プログラムとして、ヘルスチェック、カメラ、HTTP webhook、Discord / LINE / Twitter bot、OpenAI API 連携を担当します。

`root/` 側は実際のサーバ運用に必要な GROWI、バックアップ、cron、DB、証明書まわりの設定とスクリプトを担います。

`docs/` はそれらを運用するための手順書、メモ、GitHub Pages 用ドキュメントの入口です。
