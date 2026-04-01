# yappy 家の管理システム

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

## ノート

* Raspberry Pi
  * [Raspberry Pi セットアップ](./note/rpi_linux/setup_pi.md)
  * [Raspberry Pi バックアップからの復旧](./note/rpi_linux/recovery.md)
* Linux
  * [ユーザ管理](./note/rpi_linux/user.md)
  * [追加ストレージ](./note/rpi_linux/storage.md)
  * [systemd](./note/rpi_linux/systemd.md)
  * [Lighttpd](./note/rpi_linux/lighttpd.md)
  * [Docker](./note/rpi_linux/docker.md)
* Windows
  * [SSH トンネルとリモートデスクトップ、Wake on LAN](./note/windows/remote.md)
* Rust
  * [Rust ツール類](./note/rust/tools.md)
  * [Rust テクニカルノート](./note/rust/technote.md)
* Web API
  * [OpenAI API ノート](./note/webapi/openai.md)
  * (旧情報) [Twitter API ノート](./note/webapi/twitter.md)
