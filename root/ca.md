# CA (認証局) の構築と証明書の発行
openssl が入っていないなら入れる。
CA.pl というスクリプトがインストールされているのでそれを使う。
(トラブルが起きやすく、中身は大したことをしていないので、
実際のところは中身を読んで openssl コマンドを手で打つのがおすすめなのかもしれない。)
`man ca.pl` に公式ドキュメントが存在する。

```
/usr/lib/ssl/misc/CA.pl
/usr/lib/ssl/openssl.cnf
```

## CA の構築
CA の生成は少しだけ複雑なので頼ってもいいかもしれない。
パスフレーズなしにするには以下のようにする。
```
./CA.pl -newca -extra-req "-nodes"
```

何も入れずに Enter するとデフォルト値になる。
空文字列にしたい場合は "." と入力する。

* private/cakey.pem
  * CA の秘密鍵。もちろん流出しないよう気をつける。
* cacert.pem
  * CA の自己署名証明書。root CA ならこれを使う。
* careq.pem
  * CSR。他の CA に署名してもらうためにはこれを渡す。
  これに自分で署名したものが cacert.pem。

## キーペアの生成と CSR の作成
```
./CA.pl -newreq-nodes
```
こちらもプロンプトに従っていろいろ入力する。
何も入れずに Enter するとデフォルト値になる。
空文字列にしたい場合は "." と入力する。

* newkey.pem
  * 秘密鍵。お相手に渡す。
* newreq.pem
  * 対応する公開鍵に対する CSR。これに CA で署名する。

## 署名
```
./CA.pl -sign
```
再度証明書の内容が表示されるので OK ならば CA の秘密鍵で署名する。

## 形式変換
```
./CA.pl -pkcs12
```
ブラウザ等で簡単にインポートできる形式に変換する。
これをお相手に渡す。

## Revoke
```
./CA.pl -revoke <certfile>
```

## CRL 発行
```
./CA.pl -crl
```

# lighttpd への設定
https://redmine.lighttpd.net/projects/lighttpd/wiki/Docs

```
# 10-ssl.conf
$SERVER["socket"] == "0.0.0.0:443" {
  # TLS を有効にする
  ssl.engine  = "enable"

  # サーバ証明書の設定
  # サーバ証明書とサーバの秘密鍵をそれぞれ設定する
  ssl.pemfile = "/.../yappy.mydns.jp/cert.pem"
  ssl.privkey = "/.../yappy.mydns.jp/privkey.pem"

  # クライアント認証に使う CA
  # 先ほど作った root CA の自己署名証明書を設定する
  # 1.4.60 から ssl.verifyclient.ca-file にリネームされるらしい
  ssl.ca-file = "/.../cacert.pem"
  # クライアント認証を有効にする
  ssl.verifyclient.activate = "enable"
  # クライアント認証を強制するか
  # HTTP 通信の前に行われるのでドメイン内全体で強制することになる
  ssl.verifyclient.enforce = "disable"
  # 証明書の該当フィールドを REMOTE_USER としてエクスポートする
  # auth モジュールで認証の有無を確認するのに必要
  ssl.verifyclient.username = "SSL_CLIENT_S_DN_CN"
  # CRL
  ssl.verifyclient.ca-crl-file
```

なんか設定サンプルにしか書かれていないが、このようにすると一部のアドレスのみ
ssl のクライアント認証ができていない場合に 403 にできる。
```
# 05-auth.conf
auth.require = ( "/house/" =>
  (
    "method"  => "extern",
    "realm"   => "certificate",
    "require" => "valid-user"
  )
)
```

# 推奨暗号および鍵長の目安
デジタル庁, 総務省, 経済産業省
https://www.cryptrec.go.jp/list.html
