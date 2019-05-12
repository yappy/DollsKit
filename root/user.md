# ユーザアカウント管理

## 追加
```
$ useradd -s /bin/bash [-u <UID>] <NAME>
```
なぜか自分のシェルも chsh できないようなので root から bash にしてあげる。

## ssh 対応
```
# (そのユーザになって)
# ホームディレクトリで
$ cd
# .ssh ディレクトリを作成
$ mkdir .ssh
# パーミッションを自分だけアクセスできるように変更 u rwx
$ chmod 700 .ssh
# 中に入る
$ cd .ssh
# authorized_keys ファイル作成
$ touch authorized_keys
# パーミッションを自分だけ読み書きできるように変更 u rw
$ chmod 600 authorized_keys
```
その後、公開鍵を authorized_keys に追加する。

## sudoer にする
```
$ usermod -aG sudo <USER>
```
sudo グループが sudoers に書かれているのでそのグループに追加すれば OK。

```
$nano /etc/sudoers.d/010_pi-nopasswd
```
パスワードを聞かれたくない場合はこれを真似して追加する。
