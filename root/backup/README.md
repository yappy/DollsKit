# Pocket ID バックアップと復元

## バックアップ

`bkup_pocketid.py` は `bkup_growi.py` と同様に、圧縮前データを `/mnt/bkup/pocketid/dump/` に作成し、`/mnt/bkup/pocketid/archive/` に `pocketid_<host>_<timestamp>.tar.bz2` を作ります。dump は volume の `data/` と `/root/pocketid/pocketid.env` です。次の実行時に dump を作り直すため、直近の圧縮前データが残ります。volume の整合性を保つため、Pocket ID の systemd service が稼働中なら dump 取得中だけ停止し、成功・失敗にかかわらず再起動します。GROWI の service は操作しません。

保存先と dump は root のみ読み書き可能にし、環境ファイルの group/other 読み取り権限がある場合はバックアップを中断します。圧縮ファイルも mode 0600 で作られます。ローカルは直近 30 世代、rclone remote `pcloud_enc` は直近 5 世代を保持します。remote 名、保存先、世代数は `RCLONE_REMOTE`、`RCLONE_DST`、`KEEP_COUNT`、`RCLONE_KEEP_COUNT` で変更できます。空の `RCLONE_REMOTE` は転送を無効にします。

定期実行は `root/cron/pocketid_bkup` を `/etc/cron.weekly/pocketid-bkup` など run-parts に認識される名前で配置します。ログは `/root/log/pocketid_bkup.log` と `/root/log/pocketid_bkup_err.log` に追記されます。マウント確認、Docker、圧縮、転送の失敗は非 0 で終了し、cron のエラー記録に残ります。

## 復元

復元先で Docker、Compose 定義、`/root/pocketid` を用意します。復元元ホストの Compose 定義と互換な Pocket ID イメージを使い、空の volume へ復元してください。

```bash
sudo systemctl stop pocketid.service
sudo install -d -m 700 /root/pocketid
sudo install -d -m 700 /root/pocketid-restore
sudo tar -xjf /mnt/bkup/pocketid/archive/pocketid_<host>_<timestamp>.tar.bz2 \
  -C /root/pocketid-restore
sudo install -m 600 /root/pocketid-restore/pocketid.env /root/pocketid/pocketid.env
docker volume create pocket-id_pocket_id_data
docker run --rm \
  -v pocket-id_pocket_id_data:/target \
  -v /root/pocketid-restore/data:/source:ro \
  busybox:latest sh -c 'cp -a /source/. /target/'
sudo systemctl start pocketid.service
```

Compose が作る volume 名が異なる場合は、`docker volume ls` と `docker volume inspect` で実際の名前を確認し、上記の `docker volume create` と `-v` の両方を置き換えます。既存データのある volume へ重ねて復元せず、退避または新規 volume を使います。起動後に管理画面で既存ユーザー、パスキー認証、OIDC クライアントと redirect URI が保持されていることを確認します。検証後、root 専用の一時展開先 `/root/pocketid-restore` を削除します。

この専用アーカイブはデータ volume と encryption key を一体で戻すためのものです。ホスト全体バックアップには既に含まれる lighttpd 設定・証明書と Pocket ID env の通常復旧には、ホスト全体バックアップも利用できます。
