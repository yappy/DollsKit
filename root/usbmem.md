# USB メモリのセットアップ

## USB メモリの確認
```
$ dmesg
$ fdisk -l
/dev/sd[a][1]
```
a,b,c,... がデバイス番号
1,2,3,... がパーティション番号

## パーティションの削除、再作成
買ってきたものは多分 fat32 のパーティションが1つ存在する
```
$ fdisk /dev/sd[a]
ヘルプに従って進む (p 情報表示)

d パーティション削除
n パーティション新規作成
w 実際に書き込み
```

## パーティションを ext4 でフォーマット
```
$ mkfs -t ext4 /dev/sd[a][1]
```
途中本当にいいか聞かれるのでリターンキー1回

## UUID の確認
```
$ blkid
```

## マウントポイントの作成
```
$ mkdir /media/usbbkup
```

## 起動時 (または mount -a 時) に USB メモリをマウント
/etc/fstab に追加。
バックアップしたい / (ext4) のマウント設定に合わせるとよいと思う。
抜けている状態でブート失敗になるとよくないので nofail を付けた方がよいと思う。

```
PARTUUID=6c586e13-01  /boot           vfat    defaults          0       2
PARTUUID=6c586e13-02  /               ext4    defaults,noatime  0       1
UUID=<uuid>           /media/usbbkup  ext4    defaults,noatime,nofail 0 0
```

ブート時に死なないか以下で確認すること。
```
$ mount -a
```

## (主にスクリプト等からの)マウント状態の確認
```
$ mountpoint <path>
```

## 取り外すとき
```
$ umount /media/usbbkup
```

## つけなおしたとき
```
$ mount -a
```

## マウント状態の確認(ファイルシステムタイプつき)
```
$ df -T
```
