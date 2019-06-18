# yappy 家の管理システム全景

工事中！

## 管理プログラム
![管理プログラム](./shanghai.png)

### タスクサーバ
1分ごとに時刻をチェックしタスクを実行する。

### Web Server (back)
HTTP サーバ機能を含んでおり、入口の lighttpd からリバースプロキシでつないでいる。


## Web Server (front)
HTTP サーバとして lighttpd を稼働。


## 自動バックアップ
`root/usbmem.md` USB メモリを ext4 で再フォーマットし、fstab で起動時にマウントする。

`root/bkup.sh` cron によるシェルスクリプトの自動実行。


## 自動アップデート
unattended-upgrades によりセキュリティアップデートを自動的に適用し再起動する。
