# GROWI docker compose definition

## git submodule

GROWI 公式の docker compose テンプレート growi-docker-compose を
submodule として登録しています。
git clone, pull しただけでは更新されません。

```sh
# 1回でよい
git submodule init
# 更新があったら毎回
git submodule update

# 同時にやる
git submodule update --init
# submodule の中の submodule も含める
# 何も考えたくない人向け
git submodule update --init --recursive

# clone 時に一緒にやってしまう
git clone --recursive
```

なおあまりに面倒な上に update を忘れると壊れるので以下の設定がおすすめ。
ただし最初の一回は避けられないかも？

```sh
git config --global submodule.recurse true
```

## GROWI v8 へのアップグレード

公式 Compose submodule を v8 対応版へ更新し、GROWI イメージを `growilabs/growi:8` に
切り替えています。2026-10-10 時点の公式最新リリースは 8.0.7 です。

v8 は MongoDB の change stream を使うため、MongoDB を replica set として起動する必要が
あります。Compose 設定は単一ノードの `rs0` を起動時に初期化します。既存の standalone
データを使う環境では、更新前に MongoDB のバックアップを取得し、復元できることを確認して
ください。更新後の AI 連携は設定方法が変わっているため、旧 AI 連携を利用している場合は
再設定が必要です。Elasticsearch v7 は非対応になりましたが、この構成は v9 を使用します。

詳細は[公式 v8 アップグレードガイド](https://docs.growi.org/ja/admin-guide/upgrading/80x.html)
を参照してください。

## docker compose

`*.yaml`

### PASSWORD_SEED

Compose の上書き設定で `PASSWORD_SEED` を必須にし、ホスト上の
`/root/growi/growi.env` から読み込みます。このファイルは Git の管理対象にせず、
root のみが読めるようにしてください（所有者 `root:root`、権限 `0600`）。
設定を移す際は、まず現在の運用値をそのまま設定してください。値を変えると、
既存ユーザーがローカルパスワードでログインできなくなる可能性があります。

GROWI の systemd unit と MongoDB のバックアップ・復元スクリプトは、このファイルを
Docker Compose に渡します。unit の再読み込みや再起動より前に作成してください。

手動で Compose コマンドを実行する場合は、同じ設定を使うラッパーを利用できます。
たとえばアプリのログを追うには、GROWI のディレクトリで次を実行します。

```sh
./compose.sh logs -f app
```

展開後の環境変数を表示せずに Compose 設定を検証するには、GROWI のディレクトリで
次を実行します。

```sh
docker compose --env-file /root/growi/growi.env \
  -f growi-docker-compose/docker-compose.yml \
  -f compose.yaml config --quiet
```

## Pocket ID OIDC

Pocket ID は別 Compose プロジェクトとして起動します。GROWI の OIDC issuer には
`https://id.yappy-house.com` を設定し、Pocket ID のクライアントには GROWI 管理画面に
表示される Callback URL を登録してください。`APP_SITE_URL` は引き続き
`https://wiki.yappy-house.com` です。

GROWI の `app` コンテナには `id.yappy-house.com:host-gateway` を設定しています。
これによりコンテナから公開 issuer のホスト名へ接続するときだけホストへ到達し、ホスト上の
lighttpd が Pocket ID に転送します。証明書検証と OIDC issuer の一致を保つため、URL は
内部 IP や別名に置き換えません。Compose プロジェクト間の共有ネットワークや `dns:` の追加も
不要です。Compose 設定の反映には GROWI `app` コンテナの再作成が必要です。

Pocket ID 側の起動方法と lighttpd の設定例は [`../pocketid/README.md`](../pocketid/README.md)
を参照してください。

## systemd service definition

`*.service`
