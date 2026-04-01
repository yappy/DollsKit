# Lighttpd - Web Server 関連

## Lighttpd

Apache より軽さ重視のウェブサーバー。

```sh
# install
sudo apt install lighttpd
# 設定
cd /etc/lighttpd
# conf-available/* を編集する
# 以下のコマンドでシンボリックリンクを conf-enabled/*` に生成削除する
# conf-enabled に存在するファイルが読み込まれる
lighttpd-enable-mod
lighttpd-disable-mod
# 設定を反映
systemctl reload lighttpd.service
```

以下などを必要に応じて変更しながら enable する。

* accesslog
* userdir
  * `~user/public_html/` 以下を `/~user/` 以下に公開するやつ
* dir-listing
  * ディレクトリにアクセスされた場合に中身一覧ページを自動で返す
* fastcgi-php-fpm
* CGI の stderr
  * `server.breakagelog = "/var/log/lighttpd/stderr.log"`

### Lighttpd 設定ファイル

場所: `/etc/lighttpd/`

設定ファイルの文法はやや見慣れない雰囲気なので、ノーヒントで読むと解読しづらい。
公式チュートリアルは読んだ方がよい。

5分や10分でわかるらしい:
<https://redmine.lighttpd.net/projects/lighttpd/wiki/TutorialConfiguration>

条件分岐に if を書かないのがポイント。なお else は書く模様。
頭の中で if をつければ多分読める。
HTTP リクエストの内容に応じて設定値を切り替えられる。

```perl
$HTTP["host"] == "example.org" {
  # options specific to example.org
  expire.url = ( "" => "access plus 25 hours" )
} else $HTTP["host"] == "static.example.org" {
  # options specific to static.example.org
  expire.url = ( "" => "access plus 2 weeks" )
} else $HTTP["host"] =~ "" {
  # options applied to any other vhosts present on this ip
  # ie. default options
  expire.url = ( "" => "access plus 3 hours" )
}
```

* `$HTTP["scheme"]`
  * "http" ot "https"
* `$HTTP["host"]`
  * 複数のドメイン名からアクセス可能にし、サーバーで動作を切り替える
    virtual host 機能を実現したい場合に。
* `$HTTP["url"]`
  * host や querystring を含まない、パスの部分。
  * 前方マッチで何かやりたくなることが多いはず。
* `$HTTP["querystring"]`

設定ファイルをミスって起動できなくなってしまったら、`lighttpd -tt -f <FILE>` で
エラー箇所を特定する。

```sh
lighttpd:
 -f <name>  filename of the config-file
 -t         test config-file syntax, then exit
 -tt        test config-file syntax, load and init modules, then exit
```

### リバースプロキシ / Virtual Host

`10-proxy.conf`

URL path が `$HTTP["url"]` に入っているので文字列マッチして判定する。

```perl
# Rust 版向け
$HTTP["url"] =~ "^/rhouse" {
  proxy.server  = ( "" => ( ( "host" => "127.0.0.1", "port" => 8899 )))
}
```

URL path はそのまま渡されるが、前の方を書き換えたくなりがち。
しかし残念ながら少々難しい。

<https://serverfault.com/questions/135770/rewrite-url-before-passing-to-proxy-lighttpd>

"proxy.header" - "map-urlpath" を使うとうまくできるのかも。(1.4.46 or later)

apt に入っているバージョンが古くて苦しいこともあったが、
こういうの無いのかよ欲しいなあという機能はしばらく待っていると
しっかり追加される印象がある。

```perl
$HTTP["url"] !~ "/.well-known/acme-challenge/" {
  $HTTP["host"] =~ "^wiki." {
    proxy.server  = ( "" => ( ( "host" => "127.0.0.1", "port" => 3000 )))
    proxy.header = (
      "upgrade" => "enable",
    )
  }
}
```

複数のドメイン名からアクセス可能にしておけば、ホスト名の文字列マッチで
リバースプロキシすれば Virtual Host を実現できる。
ただし Let's Encrypt のチャレンジリクエストが失敗してしまうので、
URL path で調べて除外すれば回避できる。

プロキシは HTTP ヘッダの転送で問題が起きやすい。
"proxy.header" のマニュアルを参考に頑張れば何とかなることが多い…と思う。
例えば、upgrade (TCP コネクションを別プロトコルに切り替える) 対応オプションで
web socket が動かない問題は修正できる。

## SSL/TLS (Let's Encrypt)

`sudo apt install certbot`

http サーバ稼働状態でドメインと webroot (/var/www/html/ 的な位置) を
入力するだけで自動的にドメイン証明書を作ってくれる。

ディストリビューションによるのかもしれないが、Debian apt だと
`/etc/cron.d/certbot` がインストールされ、cron で自動更新設定もしてくれる。

`sudo certbot`

```text
Certbot doesn't know how to automatically configure the web server on this system. However, it can still get a certificate for you. Please run "certbot certonly" to do so. You'll need to manually configure your web server to use the resulting certificate.
```

何かよく分かんないけど証明書がもらえればそれでよいので certonly を実行する。
apache や nginx だと色々自動でやってくれるという噂もある。

`sudo certbot certonly`

lighttpd が動いているなら webroot は /var/www/html/ なので、
そこを指定すれば後は自動でチャレンジリクエストを行ってくれる。

```text
How would you like to authenticate with the ACME CA?
-------------------------------------------------------------------------------
1: Place files in webroot directory (webroot)
2: Spin up a temporary webserver (standalone)
-------------------------------------------------------------------------------
Select the appropriate number [1-2] then [enter] (press 'c' to cancel): 1
Enter email address (used for urgent renewal and security notices) (Enter 'c' to
cancel): (メール)

-------------------------------------------------------------------------------
Please read the Terms of Service at
https://letsencrypt.org/documents/LE-SA-v1.2-November-15-2017.pdf. You must
agree in order to register with the ACME server at
https://acme-v01.api.letsencrypt.org/directory
-------------------------------------------------------------------------------
(A)gree/(C)ancel: a
Please enter in your domain name(s) (comma and/or space separated)  (Enter 'c'
to cancel): (ドメイン名)
Obtaining a new certificate
Performing the following challenges:
http-01 challenge for (ドメイン名)

Select the webroot for (ドメイン名):
-------------------------------------------------------------------------------
1: Enter a new webroot
-------------------------------------------------------------------------------
Press 1 [enter] to confirm the selection (press 'c' to cancel): 1
Input the webroot for yappy.mydns.jp: (Enter 'c' to cancel):/var/www/html/
(lighttpd のデフォルトドキュメントルートの場合)
Waiting for verification...
Cleaning up challenges
Generating key (2048 bits): /etc/letsencrypt/keys/0000_key-certbot.pem
Creating CSR: /etc/letsencrypt/csr/0000_csr-certbot.pem

IMPORTANT NOTES:
 - Congratulations! Your certificate and chain have been saved at
   /etc/letsencrypt/live/(ドメイン名)/fullchain.pem. Your cert will
   expire on 2019-01-01. To obtain a new or tweaked version of this
   certificate in the future, simply run certbot again. To
   non-interactively renew *all* of your certificates, run "certbot
   renew"
 - If you like Certbot, please consider supporting our work by:

   Donating to ISRG / Let's Encrypt:   https://letsencrypt.org/donate
   Donating to EFF:                    https://eff.org/donate-le
```

成功すると、`/etc/letsencrypt/live/(domain)` にファイルができている。

* `privkey.pem`  : the private key for your certificate.
* `fullchain.pem`: the certificate file used in most server software.
* `chain.pem`    : used for OCSP stapling in Nginx >=1.3.7.
* `cert.pem`     : will break many server configurations, and should not be used
  without reading further documentation (see link below).

`/etc/cron.d/certbot` に cronjob が登録されている。
12 時間ごとに自動で renew してくれるらしい？

* lighttpd に設定する。
  * `/etc/lighttpd/conf-available/10-ssl.conf` を使う。
  * `sudo lighttpd-enable-mod (xx- と .conf を除いた名前)`
  * `sudo systemctl reload lighttpd`
  * 昔は手動で結合する必要があったが、lighttpd のアップデートで必要なくなった。

```text
ssl.pemfile = "/etc/letsencrypt/live/yappy.mydns.jp/fullchain.pem"
ssl.privkey = "/etc/letsencrypt/live/yappy.mydns.jp/privkey.pem"
# なんかデフォルトだと TLS 1.2 が弾かれる。1.3 は OK。
# TLS 1.3 の方がよいのも TLS 1.1 以下は滅ぼした方がよいのも理解するが、
# TLS 1.2 無効はやりすぎで害が上回ると思う…。
ssl.openssl.ssl-conf-cmd += ("MinProtocol" => "TLSv1.2")
```

セキュリティや設定の確認は Qualys SSL LABS で診断してもらうのがおすすめらしい。
ドメインを入れるだけで色々とチェックしてくれる。

<https://www.ssllabs.com/ssltest/>

### 自動更新

Let's Encrypt の証明書の有効期限は3か月。
※45日に短縮されるとの話あり。

`/etc/cron.d/certbot` にいい感じの cron job が登録されている。
1日2回、ランダムな時間待ってから renew 処理が走るようになっている。
1日2回やるのはネットワークや相手サーバーの調子が悪いかもしれないから。
ランダムな時間待つのは相手サーバーに負荷をかけないため。

`certbot renew` を実行すると、更新期間前の場合は何もせずに成功する。

```sh
$ certbot renew
Saving debug log to /var/log/letsencrypt/letsencrypt.log

- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
Processing /etc/letsencrypt/renewal/yappy.mydns.jp.conf
- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
Certificate not yet due for renewal
```

デフォルトでは1か月前から更新できるようになる。
※将来的に短縮されるかも。
`/etc/letsencrypt/renewal/<domain>` で `renew_before_expiry` オプションを
設定すれば変更できるようだが、そのままを推奨。

`--dry-run` を指定すれば更新期間前でも更新をシミュレートできる。

```sh
certbot renew --dry-run
```

`--force-renewal` を指定すれば更新期間を無視して必ず更新する。
テストしたい場合に便利。

```sh
certbot renew --force-renewal
```

ただし更新期間を絞っているのはサーバー側の負荷を下げるためなので、
自動化されたコードからこのオプションを使うのは非推奨。

### 証明書の失効と削除

サブドメインを変えたので revoke & delete して証明書を取り直したい場合。

```sh
certbot revoke --cert-path /path/to/cert.pem
```

Yes と答え続ければ revoke 後に削除もやってくれる。

## PHP

* `apt install php-cgi`
  * これは fastcgi じゃないらしい…。
* `apt install php-fpm`
  * FPM (FastCGI Process Manager) は、PHP における FastCGI 実装です。
    (主に)高負荷のサイトで有用な機能が含まれています。
  * Apache ではこちらが標準になったとか。
    セキュリティと性能の両面でこちらの方がいいかもしれない。
  * ただしプロセスが残るので設定ファイルを書き換えても元のが残りそう。
    `service php-fpm8.4 force-reload` 等が必要と思われる。
* `/etc/php/.../php.ini`
* デフォルトのアップロードサイズ制限は厳しいので適切に設定し直す。
  * memory_limit
  * post_max_size
  * upload_max_filesize
* `service php8.4-fpm restart`
* PukiWiki が動かない時は mb (multi byte) ライブラリ不足。
  * `apt install php-mbstring`
  * `extension=mbstring` の行をコメント解除する。

```txt
PHP Fatal error:
Uncaught Error: Call to undefined function mb_strrpos() in ...php:XXX
```

## PukiWiki

懐かしさはあるが、現代のセキュリティに対応できているかというと不安になる。
~~Wordpress よりまし。~~

<https://pukiwiki.sourceforge.io/>

海外からめちゃくちゃ荒らされる (残念だが当然) ので認証は必須。
この認証で大丈夫なのかは不明。

```php
// pukiwiki.ini.php

// Title of your Wikisite (Name this)
// Also used as RSS feed's channel name etc
$page_title = 'PukiWiki';
// Site admin's name (CHANGE THIS)
$modifier = 'anonymous';
// Site admin's Web page (CHANGE THIS)
$modifierlink = 'http://pukiwiki.example.com/';

// User definition
$auth_users = array(...);
// Edit auth (0:Disable, 1:Enable)
$edit_auth = 0;
```

ファイルアップロードに関してはおそらく `php.ini` の設定も必要。

```php
// plugin/attach.inc.php

// Max file size for upload on script of PukiWikiX_FILESIZE
define('PLUGIN_ATTACH_MAX_FILESIZE', (1024 * 1024)); // default: 1MB
```
