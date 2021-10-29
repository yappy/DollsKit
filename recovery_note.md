# 復旧 / バージョンアップ / クリーンアップ 手順

## 初期化
setup_note.md を参考に初期イメージを SD card に焼いて ssh を確立する。


### ssh host の鍵変更による影響
再インストールして ssh の認証情報をバックアップから復旧するだけだと、
クライアント側の known_hosts に保存されているホスト鍵が不一致を起こす。
メッセージの通りに ~/.ssh/known_hosts から該当行を削除すればよい。
```
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@       WARNING: POSSIBLE DNS SPOOFING DETECTED!          @
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
The ECDSA host key for [xxx.xxx.jp]:22 has changed,
and the key for the corresponding IP address [xxx.xxx.xxx.xxx]:22
is unknown. This could either mean that
DNS SPOOFING is happening or the IP address for the host
and its host key have changed at the same time.
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
IT IS POSSIBLE THAT SOMEONE IS DOING SOMETHING NASTY!
Someone could be eavesdropping on you right now (man-in-the-middle attack)!
It is also possible that a host key has just been changed.
The fingerprint for the ECDSA key sent by the remote host is
SHA256:xxxxxxxxxxxxxxxx.
Please contact your system administrator.
Add correct host key in /home/username/.ssh/known_hosts to get rid of this message.
Offending ECDSA key in /home/username/.ssh/known_hosts:1
  remove with:
  ssh-keygen -f "/home/username/.ssh/known_hosts" -R "[xxx.xxx.jp]:22"
ECDSA host key for [xxx.xxx.jp]:22 has changed and you have requested strict checking.
Host key verification failed.
```


## バックアップメディアのマウント
USB メモリなら `root/usbmem.md` を参考にマウントしてバックアップデータを得る。


### rsync
丸ごと転送便利コマンド。
`rsync -a <SRC> <DST>`

* `-a` オーナーやパーミッション
* `-v` 詳細情報を表示する。`-vvv` のように複数回指定するほど詳しくなる。
  2個くらいつけておくのがおすすめ。
* `--delete` 転送先にファイルがなければ消す。完全に同期したい場合に。
* `--backup` 上書き/消す場合にバックアップを作成する。
* `-n` dry-run


## まるごとリストアする場合
バックアップデータをそのまま / に上書き転送すればよい。

新バージョンにクリーンアップしながら復旧したい場合は以下の手順で少しずつ復旧する。


## ユーザの復旧
バックアップから rsync -a あたりで /home をコピーする。

以下のファイルをバックアップから復旧する。
それぞれ vipw, vigr, visudo を使ってチェックをかけながら保存するのが定石だが、
初期イメージ焼きからやり直す覚悟があるなら上書きコピーする。
(`rsync -a --backup` を使うと多少マシかもしれない)

* /etc/
  * passwd
  * shadow
  * group
  * gshadow
  * sudoers
  * sudoers.d/*
    * 特に 010_pi-nopasswd


## SSH の復旧
設定ファイルは /etc/ssh/sshd_config だが、トラブルの残っているユーザしか
入れなくなってしまうと詰むので、バックアップから上書きするのではなく
diff を参考にしながら改めて少しずつ編集し直した方がよいかもしれない。


## crontab の復旧
crontab で設定したデータは `/var/spool/cron/` 以下にある。
