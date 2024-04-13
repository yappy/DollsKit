# ユーザアカウント管理

## 秘密鍵/公開鍵ペアの作成方法

```sh
ssh-keygen -C <Comment>
# アルゴリズムと強度をデフォルトから変える場合
ssh-keygen -t rsa -b 2048 -C <Comment>
```

画面にも出るが、`.ssh/` 以下に出力される。

* `id_rsa`: 秘密鍵
  * 取り扱い注意
* `id_rsa.pub`: 公開鍵

## 追加

```sh
useradd -s /bin/bash [-u <UID>] <NAME>
```

なぜか自分のシェルも chsh できないようなので root から bash にしてあげる。

```sh
adduser <NAME>
```

対話式で、パスワードの設定もしてくれるし bash にしてくれるらしいしで
これが使えるならばこちらの方が楽。ホームディレクトリも作ってくれる。

## 削除

```sh
userdel -r <NAME>
```

`-r` はホームディレクトリとメールスプールも削除する。

## ssh 対応

```sh
# (そのユーザになって)
# ホームディレクトリで
cd
# .ssh ディレクトリを作成
mkdir .ssh
# パーミッションを自分だけアクセスできるように変更 u rwx
chmod 700 .ssh
# 中に入る
cd .ssh
# authorized_keys ファイル作成
touch authorized_keys
# パーミッションを自分だけ読み書きできるように変更 u rw
chmod 600 authorized_keys
```

その後、公開鍵を authorized_keys に追加する。

## sudoer にする

```sh
usermod -aG sudo <USER>
```

sudo グループが sudoers に書かれているのでそのグループに追加すれば OK。

```sh
nano /etc/sudoers.d/010_pi-nopasswd
```

パスワードを聞かれたくない場合はこれを真似して追加する。

## バックアップからの復旧

* /etc/
  * passwd
  * shadow
  * group
  * gshadow
  * sudoers
  * sudoers.d/
    * 010_pi-nopasswd
