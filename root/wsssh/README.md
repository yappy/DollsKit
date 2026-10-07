# SSH over WebSocket

外出先のネットワークで SSH ポートが遮断され、HTTPS の 443 番だけ通る場合に、
Lighttpd の WebSocket リバースプロキシを経由して同じサーバー上の `sshd` に接続する。
接続先の SSH ポートは `56789` とする。

## 通信経路

```text
OpenSSH
  └─ ProxyCommand: wstunnel client
       └─ WSS / TCP 443
            └─ Lighttpd /ssh-ws
                 └─ WebSocket / 127.0.0.1:18080
                      └─ wstunnel server
                           └─ TCP 127.0.0.1:56789 (sshd のみ)
```

TLS は既存の Lighttpd が終端する。wstunnel は loopback 上の平文 WebSocket で受け、
転送先を `127.0.0.1:56789` のみに制限する。ログイン認証は SSH 公開鍵認証を使う。

## 導入前の確認

1. 公開鍵で SSH ログインできることを確認する。既存の外部 SSH 経路は作業中も残す。
1. `lighttpd -v` で Lighttpd が 1.4.46 以降であること、
  HTTPS/443 と証明書がすでに動作していることを確認する。
1. `127.0.0.1:18080` が未使用であることを確認する。

   ```sh
   sudo ss -lntp | rg ':18080\b'
   ```

   何も表示されなければ未使用。
   使用中なら以下の例の `18080` を未使用のポートに統一して置き換える。

## サーバーへの wstunnel 導入

[GitHub wstunnel](https://github.com/erebe/wstunnel)

このディレクトリでソースからビルドする場合は、`./build-wstunnel.sh` を実行する。
スクリプトの `WSTUNNEL_TAG` で指定したタグ（初期値は `v11.0.0`）を
`wstunnel/` に checkout し、`wstunnel/target/release/wstunnel` を生成する。
clone したディレクトリをビルド成果物ごと削除する場合は、
`./clean-wstunnel.sh` を実行する。
サーバーでは、`sudo ./install-wstunnel.sh` で生成したバイナリを
`/opt/wstunnel/wstunnel` に配置する。配置先を変更する場合は、
`sudo ./install-wstunnel.sh /path/to/wstunnel` のように引数で指定する。

サービス用のユーザーとグループは `DynamicUser=yes` により
サービスの起動時に systemd が割り当て、停止時に解放する。
手動で作成する必要はない。

同梱の [`wstunnel-ssh.service`](wstunnel-ssh.service) へのシンボリックリンクを
`/etc/systemd/system/` に作成する。以下はこのディレクトリで実行する。
リポジトリを移動するとリンクが切れるため、配置場所を固定しておく。
バイナリの配置先を変更した場合は、
サービスファイルの `ExecStart` も合わせて変更する。

```sh
sudo ln -s "$(realpath wstunnel-ssh.service)" /etc/systemd/system/wstunnel-ssh.service
```

サービスを起動して状態を確認する。

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now wstunnel-ssh.service
sudo systemctl status wstunnel-ssh.service
sudo journalctl -u wstunnel-ssh.service
```

## Lighttpd の設定

`mod_proxy` が未有効なら有効にする。

```sh
sudo lighttpd-enable-mod proxy
```

`/etc/lighttpd/conf-available/10-proxy.conf` など、既存の proxy 設定に追加する。
`server.modules` が設定ファイルで明示管理されている場合は `mod_proxy` も追加する。

```perl
$HTTP["url"] =~ "^/ssh-ws" {
  proxy.server = ( "" => ((
    "host" => "127.0.0.1",
    "port" => 18080,
  )))
  proxy.header = ( "upgrade" => "enable" )
  server.max-read-idle := 300
  server.max-write-idle := 300
}
```

URI は wstunnel にそのまま転送する。wstunnel の ping は既定で 30 秒間隔なので、
Lighttpd の read/write idle timeout は 300 秒とし、
長時間アイドル状態の SSH セッションを切断しないようにする。
条件内で既定値を上書きするため、`:=` を使う。

構文検査に成功してから reload する。

```sh
sudo lighttpd -tt -f /etc/lighttpd/lighttpd.conf
sudo systemctl reload lighttpd.service
```

## Linux / macOS クライアント

[GitHub wstunnel](https://github.com/erebe/wstunnel)

以下の例の `example.org` は Lighttpd の HTTPS ドメイン、
`yappy` は SSH ユーザー名に置き換える。

`~/.ssh/config` に追加する。

```sshconfig
Host home-ws
    HostName 127.0.0.1
    User yappy
    Port 56789
    IdentityFile ~/.ssh/id_ed25519
    IdentitiesOnly yes
    ProxyCommand wstunnel client --log-lvl=off --tls-verify-certificate --http-upgrade-path-prefix ssh-ws -L stdio://%h:%p wss://example.org
```

`--tls-verify-certificate` は必ず指定する。SSH 接続を開始する。

```sh
ssh home-ws
```

OpenSSH のホスト鍵確認は通常どおり行う。

## 動作確認

1. サーバー上で `wstunnel-ssh.service` が active であり、`127.0.0.1:18080` のみで待ち受けることを確認する。
2. Lighttpd の通常の Web ページと HTTPS 証明書が引き続き正常であることを確認する。
3. 外部の「443 番のみ通る」ネットワークから `ssh home-ws` を実行する。
4. 誤った SSH 鍵ではログインできないことを確認する。
5. wstunnel の転送先を別ポートに変更した試験接続は、サーバー側の `--restrict-to` によって拒否されることを確認する。
6. 従来の外部 SSH ポート転送も接続できることを確認する。

## 障害時の確認と切り戻し

- WebSocket のハンドシェイクに失敗する場合は、Lighttpd のバージョン、`mod_proxy`、`proxy.header`、`/ssh-ws` のパス一致を確認する。
- Lighttpd が 502 を返す場合は、wstunnel サービスの状態と `127.0.0.1:18080` の待受を確認する。
- SSH の認証で失敗する場合は、`HostName` / `Port`、サーバーの `sshd` 設定、公開鍵と `IdentityFile` を確認する。
- 接続が一定時間後に切れる場合は、Lighttpd の read/write idle timeout と WebSocket ping を確認する。
- 切り戻す場合は Lighttpd の `/ssh-ws` 設定だけを外して構文検査後に reload し、wstunnel サービスを停止する。既存の外部 SSH 経路には手を加えない。
