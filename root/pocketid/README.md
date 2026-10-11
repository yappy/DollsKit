# Pocket ID

Pocket ID を独立した Docker Compose プロジェクトとして起動し、ホストの lighttpd から
`https://id.yappy-house.com` で公開する構成です。Pocket ID のポートはホストの loopback
だけに bind します。

## 環境変数

ホストに `/root/pocketid/pocketid.env` を作成し、root のみが読めるようにしてください
（所有者 `root:root`、権限 `0600`）。実値は Git に登録しません。

```dotenv
ENCRYPTION_KEY=十分に長いランダムな秘密値
```

`ENCRYPTION_KEY` は Pocket ID のデータを暗号化するために使われます。十分な長さのランダム値を
生成して設定し、バックアップにも含めてください。紛失・変更すると既存データを復号できなくなる
可能性があります。少なくとも 16 bytes が必要です。例として `openssl rand -base64 32` で
生成できます。

## 起動

```sh
./compose.sh up -d
./compose.sh logs -f pocket-id
```

systemd で管理する場合は、この unit を `/etc/systemd/system/pocket-id.service` に配置して
有効化します。

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now pocket-id.service
```

Pocket ID のデータは Compose の名前付き volume `pocket_id_data` に保存されます。バックアップ時は
この volume と `/root/pocket-id/pocket-id.env` の両方を保管してください。

## lighttpd リバースプロキシ

DNS の `id.yappy-house.com` がこのホストを指し、lighttpd 用の TLS 証明書にこのホスト名が含まれて
いる状態にしてください。lighttpd の有効な Virtual Host 設定へ次を追加します。

```perl
$HTTP["url"] !~ "/.well-known/acme-challenge/" {
  $HTTP["host"] == "id.yappy-house.com" {
    proxy.server = ( "" => ( ( "host" => "127.0.0.1", "port" => 1411 )))
  }
}
```

設定を反映します。

```sh
sudo lighttpd -tt -f /etc/lighttpd/lighttpd.conf
sudo systemctl reload lighttpd.service
```

外部ブラウザーと GROWI の両方から `https://id.yappy-house.com` を使います。GROWI の Compose 設定
には、そのホスト名だけホスト gateway に解決する `extra_hosts` を追加済みです。Pocket ID の
`APP_URL` と GROWI の OIDC issuer は、ともに `https://id.yappy-house.com` にしてください。

## GROWI OIDC クライアント

Pocket ID に OIDC クライアントを作成し、GROWI の OIDC issuer として
`https://id.yappy-house.com` を設定します。Redirect URI には GROWI の管理画面が表示する Callback
URL をそのまま登録してください。GROWI の公開 URL は `https://wiki.yappy-house.com` のままです。

既存アカウントを維持するため、OIDC ログインと既存アカウントの紐付けを確認するまで GROWI の
ID/Password ログインを有効にしておきます。確認後に必要であれば GROWI の設定画面から無効化します。
