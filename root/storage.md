# 追加ストレージ

USB メモリ、HDD、SSD 等

## ストレージデバイスの確認

```sh
dmesg
fdisk -l
# /dev/sd[a][1]
```

* a,b,c,... がデバイス番号
* 1,2,3,... がパーティション番号

## パーティションの削除、再作成 (初期化時のみ)

USB メモリの場合、買ってきたものは多分 fat32 のパーティションが1つ存在する。
他の場合もおそらく ext4 ではない。

```sh
fdisk /dev/sd[a]
# ヘルプに従って進む (p 情報表示)
# d パーティション削除
# n パーティション新規作成
# w 実際に書き込み
```

## パーティションを ext4 でフォーマット (初期化時のみ)

```sh
mkfs -t ext4 /dev/sd[a][1]
```

途中本当にいいか聞かれるのでリターンキー1回

## UUID の確認 (バックアップ復旧時も)

```sh
blkid
```

## マウントポイントの作成 (バックアップ復旧時も)

```sh
mkdir /mnt/localbkup
```

本運用前にパーミッションに注意する。

## お試しマウント

```sh
mount /mnt/localbkup /dev/sd[a][1]
df -Th
umount /mnt/localbkup
```

## 起動時 (または mount -a 時) にマウント

/etc/fstab に追加。
バックアップしたい / (ext4) のマウント設定に合わせるとよいと思う。
抜けている状態でブート失敗になるとよくないので nofail を付けた方がよいと思う。

```text
PARTUUID=6c586e13-01  /boot           vfat    defaults          0       2
PARTUUID=6c586e13-02  /               ext4    defaults,noatime  0       1
UUID=<uuid>           /mnt/backup     ext4    defaults,noatime,nofail 0 0
UUID=<uuid>           /mnt/cloud      ext4    defaults,noatime,nofail 0 0
UUID=<uuid>           /mnt/localbkup  ext4    defaults,noatime,nofail 0 0
```

1. デバイス。
1. マウント先。
1. ファイルシステム。
1. オプション。
1. dump コマンドの対象にするか。 (かなり古くからあるバックアップツールらしい)
1. 起動時の fsck 順序。root fs は 1 を推奨。それ以外は 2 を推奨。0 で無効。

* defaults: デフォルト設定。
* noauto: `mount -a` で自動マウントしない(起動時含む)。
* noatime: ファイルを触るたびに最終使用日時を更新するのをやめる。
  最近は一日くらいバッファリングしてから書き込んでいるとの噂も。
* nofail: デバイスがなくてもエラーにしない。
起動時に抜けていると起動が失敗してしまうので、外付けの場合指定を推奨。

このまま再起動した場合にブート時に死なないか以下で確認しておくこと。

```sh
mount -a
```

## (主にスクリプト等からの)マウント状態の確認

```sh
mountpoint <path>
```

## 取り外すとき

```sh
umount /mnt/localbkup
```

## つけなおしたとき

```sh
mount -a
# or
mount /mnt/localbkup
```

## マウント状態の確認(ファイルシステムタイプつき)

```sh
df -T
```

## 設定例

* 1 TB SSD
  * 256 GB: /mnt/localbkup
    * このマシンの自動バックアップ
  * 256 GB: /mnt/cloud
  * 512 GB: /mnt/backup

## 廃棄 (※作業時注意)

デバイスファイルにランダムなデータを書き込んで破壊する。

```sh
shred -v /dev/sd[a]
```

* -u: 完了後、ファイルを削除する。デバイスファイルでなく通常ファイル向け。
* -v: 進捗表示
* -n [N]: N 回乱数で上書きする。(default=3)
* -z: 最後に 0 で上書きする。
