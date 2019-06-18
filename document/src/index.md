# システム全景

## 管理プログラム

### Camera


## Web Server (front)
HTTP サーバとして lighttpd を稼働。


## 自動バックアップ
`root/usbmem.md` USB メモリを ext4 で再フォーマットし、fstab で起動時にマウントする。
`root/bkup.sh` cron によるシェルスクリプトの自動実行。


## 自動アップデート
unattended-upgrades によりセキュリティアップデートを自動的に適用し再起動する。
