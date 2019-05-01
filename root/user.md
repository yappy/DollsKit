# ユーザアカウント管理

## 追加
```
$ useradd -s /bin/bash [-u <UID>] <NAME>
```
なぜか自分のシェルも chsh できないようなので root から bash にしてあげる。

## sudoer にする
```
$ usermod -aG sudo <USER>
```
sudo グループが sudoers に書かれているのでそのグループに追加すれば OK。

```
$nano /etc/sudoers.d/010_pi-nopasswd
```
パスワードを聞かれたくない場合はこれを真似して追加する。
