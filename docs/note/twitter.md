
# Twitter API v2 (旧)

Last update: 2022/10

<https://developer.twitter.com/en/docs>

"en" を "ja" に直すと日本語のページが表示されるが、
トップページ以外は英語しかないようなので諦めて英語を読む。

これの curl あたりを読めばだいたい把握できる。

<https://developer.twitter.com/en/docs/tools-and-libraries/using-postman>

<https://documenter.getpostman.com/view/9956214/T1LMiT5U>

## 登録

Developer Portal へ bot 用のアカウントでサインイン。
最初にいくつかの質問に答え、規約に同意する。

v1.1 時代に登録したアプリは Standalone Apps として残っている。
v2 ではプロジェクトという単位配下にあるものからしか v2 API のほとんどが利用できない。
プロジェクトを新規作成し、その配下に既存アプリを登録することで
v1.1 アクセスを維持しつつ v2 対応されるようだ。
Development, Staging, Production の選択ができるが、違いは謎。

今のところ Essential, Elevated, Academic Research の3つのプランがある。
Essential は特に何もしなくてもそのまま無料で使える。
Elevated は使用目的等を頑張って英作文すると無料で使える。
Academic Research も無料で、学術研究用。

プロジェクトとその配下にアプリを作成したら、アプリの設定
authentication settings で認証トークンを作成する。

## エンドポイントの例

v2 では User ID が必要。

* Standard v1.1 endpoints:
  * <https://api.twitter.com/1.1/statuses/home_timeline>
  * <https://api.twitter.com/1.1/statuses/user_timeline>
  * <https://api.twitter.com/1.1/statuses/mention_timeline>
* Twitter API v2 endpoint:
  * <https://api.twitter.com/2/users/:id/timelines/reverse_chronological>
  * <https://api.twitter.com/2/users/:id/tweets>
  * <https://api.twitter.com/2/users/:id/mentions>

## 認証

<https://developer.twitter.com/en/docs/authentication/overview>

適当なエンドポイントに認証なしでアクセスすると 401 で失敗する。

```sh
$ curl https://api.twitter.com/2/users/by/username/yappy_y
{
  "title": "Unauthorized",
  "type": "about:blank",
  "status": 401,
  "detail": "Unauthorized"
}
```

### App Only (OAuth 2.0 Bearer Token)

<https://developer.twitter.com/en/docs/authentication/oauth-2-0>

誰でも見える public なデータは Developers Portal で生成できる Bearer Token による
アプリの認証のみで取得できるようになる。

Developers Portal のアプリのページを開き、"Keys and tokens" のタブで
"Bearer Token" を生成できる。
Revoke および再生成も可能。

HTTP header に Authorization: Bearer \<TOKEN\> を追加するだけで OK。
curl なら --header/-H オプション。

```sh
curl "https://api.twitter.com/2/tweets?ids=1261326399320715264,1278347468690915330" \
  -H "Authorization: Bearer AAAAAAAAAAAAAAAAAAAAAFnz2wAAAAAAxTmQbp%2BIHDtAhTBbyNJon%2BA72K4%3DeIaigY0QBrv6Rp8KZQQLOTpo9ubw5Jt?WRE8avbi"
```

### OAuth 1.0a User Context

<https://developer.twitter.com/en/docs/authentication/oauth-1-0a>

昔からあるいつものやつ。
そのアカウントでしか行えない操作を行える。

Twitter の解説ページには書かれていないが、
OAuth 1.0a 自体の仕様で、request body の Content-Type が
application/x-www-form-urlencoded の場合のみ署名計算の対象になる。
つまり、Twitter API v2 では POST body は application/json であるため、
この部分は署名計算の対象にしない。

それでいいのか？という気もするが、
OAuth 1.0a `3.4.1. Signature Base String` にしっかり注意が書いてある。
(AOuth の署名だけでは request body 部分の改竄検知はできない)
