# WebSocket 経由の SSH 接続

外出先のネットワークで SSH ポートが遮断され、HTTPS の 443 番だけ通る場合に、
Lighttpd の WebSocket リバースプロキシを経由して同じサーバー上の `sshd` に接続する。
接続先の SSH ポートは `56789` とする。既存の外部 SSH ポート転送は維持し、従来経路と併用する。

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
HTTP Basic 認証は追加しない。このため WebSocket 経由で SSH の認証要求は外部から到達可能であり、
SSH の公開鍵認証を有効にし、パスワード・keyboard-interactive 認証と root ログインを無効にしておく。

## 導入前の確認

1. 公開鍵で SSH ログインできることを確認する。既存の外部 SSH 経路は作業中も残す。
2. `sshd` のポートが `56789` で、`127.0.0.1:56789` に接続可能であることと、実効設定を確認する。

   ```sh
   sudo sshd -T -C user=USERNAME,host=localhost,addr=127.0.0.1,laddr=127.0.0.1,lport=56789 | rg '^(port|pubkeyauthentication|authenticationmethods|passwordauthentication|kbdinteractiveauthentication|permitrootlogin) '
   sudo ss -lntp | rg ':56789\b'
   ```

   `port 56789`、`pubkeyauthentication yes`、`authenticationmethods publickey`、
   `passwordauthentication no`、`kbdinteractiveauthentication no`、`permitrootlogin no` を確認する。
   `USERNAME` を SSH ユーザー名に置き換える。サーバー上で `ssh -p 56789 USERNAME@127.0.0.1` を実行し、loopback 経由の鍵ログインも確認する。
   現在の設定が異なる場合は、既存の SSH 接続を維持した状態で設定を調整し、鍵で再接続できてから進める。
3. `lighttpd -v` で Lighttpd が 1.4.46 以降であること、HTTPS/443 と証明書がすでに動作していることを確認する。
4. `127.0.0.1:18080` が未使用であることを確認する。

   ```sh
   sudo ss -lntp | rg ':18080\b'
   ```

   何も表示されなければ未使用。使用中なら以下の例の `18080` を未使用のポートに統一して置き換える。

## サーバーへの wstunnel 導入

[wstunnel の公式 Releases](https://github.com/erebe/wstunnel/releases)から、サーバーの CPU
アーキテクチャに合うバイナリを選ぶ。提供されているチェックサムがあれば検証し、
[公式 CLI 説明](https://github.com/erebe/wstunnel)を確認したうえでバージョンを固定し、
`/usr/local/bin/wstunnel` に配置する。

専用の非ログインユーザーを作る。

```sh
sudo adduser --system --no-create-home --group --disabled-login wstunnel
```

`/etc/systemd/system/wstunnel-ssh.service` を作成する。

```ini
[Unit]
Description=SSH over WebSocket tunnel
After=network.target

[Service]
Type=simple
User=wstunnel
Group=wstunnel
ExecStart=/usr/local/bin/wstunnel server --restrict-to 127.0.0.1:56789 ws://127.0.0.1:18080
Restart=on-failure
RestartSec=5s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true

[Install]
WantedBy=multi-user.target
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
Lighttpd の read/write idle timeout は 300 秒とし、長時間アイドル状態の SSH セッションを切断しないようにする。
条件内で既定値を上書きするため、`:=` を使う。

構文検査に成功してから reload する。

```sh
sudo lighttpd -tt -f /etc/lighttpd/lighttpd.conf
sudo systemctl reload lighttpd.service
```

## Linux / macOS クライアント

[wstunnel の公式 Releases](https://github.com/erebe/wstunnel/releases)から、クライアント端末の
OS と CPU アーキテクチャに合うバイナリをインストールし、バージョンをサーバーと記録しておく。
サーバーと同じ固定バージョンを使う。
以下の例の `example.org` は Lighttpd の HTTPS ドメイン、`yappy` は SSH ユーザー名に置き換える。

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

OpenSSH のホスト鍵確認は通常どおり行う。初回に表示される fingerprint は、信頼できる別経路で
サーバー上の `/etc/ssh/ssh_host_*_key.pub` と照合してから受け入れる。

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
