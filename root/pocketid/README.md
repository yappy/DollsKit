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

systemd で管理する場合は、この unit を `/etc/systemd/system/pocketid.service` に配置して
有効化します。

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now pocketid.service
```

Pocket ID のデータは Compose の名前付き volume `pocket_id_data` に保存されます。バックアップ時は
この volume と `/root/pocketid/pocketid.env` の両方を保管してください。

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
には、そのホスト名だけホスト gateway に解決する `extra_hosts` を追加済みです。

### 初回セットアップ

lighttpd の設定を反映して Pocket ID に HTTPS でアクセスできることを確認したら、ブラウザーで
`https://id.yappy-house.com/setup` を開き、画面の案内に従って初期管理者アカウントを作成します。
以降は `https://id.yappy-house.com` から通常のログイン画面へアクセスできます。
パスキーを登録しておきましょう。

## Pocket ID のユーザーと GROWI 用クライアント

管理者で Pocket ID にログインし、GROWI にログインさせるユーザーと OIDC クライアントを設定します。
作業中は GROWI 側にも管理者アカウントでログインし、管理ページ > `認証機構設定` > `OIDC` を開いて
Callback URL を確認してください。

1. **ユーザーグループを作成**します。`Administration` → `User Groups` → `Add Group` を開き、例として
   表示名 `GROWI users`、グループ名 `growi-users` を登録します。グループ名はトークンに含める場合に
   アプリ側から見える値です。
2. **ユーザーをグループに追加**します。`Administration` → `Users` からユーザーを開き、作成した
   `growi-users` グループに追加します。GROWI の既存アカウントと紐付ける場合は、検証済みの紐付け
   条件に合うユーザー情報を登録してください。
3. **OIDC クライアントを作成**します。`Administration` → `OIDC Clients` → `Add OIDC Client` を開き、
   名前を `GROWI` などとします。Callback URL には GROWI 管理画面に表示される値をそのまま登録します。
   GROWI はクライアントシークレットを使うため、Public Client は無効のままにします。
4. **クライアントの許可グループを設定**します。作成したクライアントの `Allowed User Groups` で
   `growi-users` を許可します。新規クライアントは初期状態ではユーザーグループが許可されていないため、
   ここを設定しないとユーザーはサインインできません。GROWI にログインさせる全ユーザーをこのグループに
   追加してください。
5. 保存後に表示される **Client ID と Client Secret** を控え、GROWI の OIDC 設定に登録します。
   Issuer は `https://id.yappy-house.com` です。GROWI の公開 URL は
   `https://wiki.yappy-house.com` のままです。
6. GROWI の `Attribute Mapping` には、Pocket ID の `OIDC Data Preview` に表示された claim 名を
   次のように設定します。

   | GROWI の項目 | Attribute Mapping |
   | --- | --- |
   | Identifier | `sub` |
   | Username | `preferred_username` |
   | Name | `display_name` |
   | Email | `email` |

   値にはプレビュー JSON の中身ではなく、左列に示した claim 名を入力します。上記は提示された
   プレビュー例に基づく設定です。Pocket ID で Name に `name` を使いたい場合は、Name のマッピングを
   `name` にしても構いません。

グループは Pocket ID 側でクライアントへのログインを許可するために使います。GROWI 内でグループ情報を
利用する設定をしない限り、`groups` scope やカスタムクレームの追加は不要です。グループ情報を OIDC
トークンに含める場合は、GROWI 側で `groups` scope を要求する設定も行ってください。

既存アカウントを維持するため、OIDC ログインと既存アカウントの紐付けを確認するまで GROWI の
ID/Password ログインを有効にしておきます。確認後に必要であれば GROWI の設定画面から無効化します。
