# yappy 家の管理システム

## ノート

* Raspberry Pi
  * [Raspberry Pi セットアップ](./note/setup_pi.md)
  * [Raspberry Pi バックアップからの復旧](./note/recovery.md)
* Linux
  * [ユーザ管理](./note/user.md)
  * [追加ストレージ](./note/storage.md)
* Windows
  * [SSH トンネルとリモートデスクトップ、Wake on LAN](./note/remote.md)
* Rust
  * [Rust ツール類](./note/rust_tools.md)
  * [Rust テクニカルノート](./note/rust_technote.md)
* Twitter (旧情報)
  * [Twitter API ノート](./note/twitter.md)

## 管理プログラム

<!--
  docs/doc/ は GitHub Ations から生成される。
  .github/workflows/doc.yml 参照
-->
[ドキュメント](./doc/shanghai/index.html)
[ドキュメント - sys](./doc/sys/index.html)
[ドキュメント - verinfo](./doc/verinfo/index.html)
[ドキュメント - utils](./doc/utils/index.html)

### タスクサーバ

1分ごとに時刻をチェックしタスクを実行する。

### Web Server (back)

HTTP サーバ機能を含んでおり、入口の lighttpd からリバースプロキシでつないでいる。

## Web Server (front)

HTTP サーバとして lighttpd を稼働。

## 自動バックアップ

`root/storage.md` USB gen 3.2 に SSD を装備。

`root/cron/bkup` cron によるシェルスクリプトの自動実行。

`root/cron/wpbkup` WordPress に関してはソースコードとデータベースを毎日バックアップ。

## 自動アップデート

unattended-upgrades によりセキュリティアップデートを自動的に適用し再起動する。
