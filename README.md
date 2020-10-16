# DollsKit

Branch master: [![Build Status](https://travis-ci.org/yappy/DollsKit.svg?branch=master)](https://travis-ci.org/yappy/DollsKit)

yappy家の管理プログラム

## ソースの入手
```
$ git clone <this_repository>
# checkout や pull で submodule に更新が入った場合は毎回実行すること
$ git submodule update --init --recursive
```

## ビルド
環境の整った人形、または PC の中で

```
$ mkdir build
$ cd build

# -GNinja で Make ではなく Ninja build を使用可能
# (未指定でも動きますが、最初にビルドタイプがツイートされます)
$ cmake [-GNinja] -DCMAKE_BUILD_TYPE=Release ..
# 以後、ccmake . で再 config 可能

$ make -j4
$ make install
or
$ ninja
$ ninja install

# dist/ に必要なファイルができる
```

## 管理プログラムの実行開始
### 設定
起動には設定ファイルが必要です。
デフォルトファイルをコピーして作成してください。
ほぼすべての機能はデフォルトでは無効になっています。
存在しないキーはデフォルトファイルの内容が使われます。
> cp config.default.json config.json

### 実行
```
$ make run
```

### daemon として実行
```
$ make start
or
$ ninja start

# kill
$ make stop
or
$ ninja stop

# 停止の中身は
$ kill `cat shanghai.pid`
```

### シグナル
* SIGINT
* SIGTERM
  * プログラムを終了します。
* SIGHUP
  * (プロセスを終了せずに) 再起動します。設定やリソースファイルのリロードに使えます。
* SIGUSR1
  * ログをフラッシュします。
    SD カード保護のため、ログはなるべくメモリに保持する設計になっています。

例
```
$ kill -SIGUSR1 `cat shanghai.pid`
```

## システム起動時に自動起動
インストール先ディレクトリに cron.txt ができます。
```
$ crontab < cron.txt
```

## 設定 (抜粋)
### System
* AllTasksFirst:
テストのため、起動直後に全タスクを一回ずつリリースします。
本番では false にしてください。

### TwitterConfig
* FakeTweet:
実際にはツイートせず、ログに出力するのみにします。
確認後、本番では true にしてください。

### Switch
SwitchBot の MAC address を文字列の配列として設定してください。

### HttpServer (lighttpd での設定例)
```
server.modules   += ( "mod_proxy" )

$HTTP["url"] =~ "^/house" {
  proxy.server  = ( "" => ( ( "host" => "127.0.0.1", "port" => 8888 )))
  proxy.header = ("map-urlpath" => ("/house/" => "/"))
}
```

## テストの実行
```
% <make or ninja> shorttest
% <make or ninja> fulltest
```

## 注意
### Build Type
CMAKE_BUILD_TYPE は Debug, Release, RelWithDebInfo, MinSizeRel が指定可能です。

### 並列 make
-jn の n の値は論理コア数に応じて適切な数を指定してください。
~~人形はメモリが少ないので -j として無制限にするとメモリ不足で死にます。~~
最近は大量メモリ搭載モデルが出てきているのでそこまででもないかもしれません。
