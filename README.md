# DollsKit

Branch master: [![Build Status](https://travis-ci.org/yappy/DollsKit.svg?branch=master)](https://travis-ci.org/yappy/DollsKit)

yappy家の管理プログラム

## ビルド
環境の整った人形、または PC の中で
```
$ mkdir build
$ cd build
$ cmake -DCMAKE_BUILD_TYPE=Release ..
$ make -j4
$ make install
```

## 管理プログラムの実行開始
```
$ make run
```

daemon として実行
```
$ make start

# kill
$ kill `cat shanghai.pid`
```

## システム起動時に自動起動
インストール先ディレクトリに cron.txt ができます。
```
$ crontab < cron.txt
```

## テストの実行
```
% make shorttest
% make fulltest
```

## 注意
### 設定
起動には設定ファイルが必要です。
デフォルトファイルをコピーして作成してください。
ほぼすべての機能はデフォルトでは無効になっています。
存在しないキーはデフォルトファイルの内容が使われます。
> config.default.json => config.json

### Build Type
CMAKE_BUILD_TYPE は Debug, Release, RelWithDebInfo, MinSizeRel が指定可能です。

### 並列 make
-jn の n の値は論理コア数に応じて適切な数を指定してください。
人形はメモリが少ないので -j として無制限にするとメモリ不足で死にます。
