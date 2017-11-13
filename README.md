# DollsKit
yappy家の管理プログラム

## ビルド
環境の整った人形の中で

> % make all

次回以降は

> % make

(詳細は Makefile 参照)

## ビルド(Windows)
C# の入った Visual Studio で DollsKit.sln を開く

> ビルド > ソリューションのビルド

## 人形の実行
```
% cd deploy
% mono Shanghai.exe
```
```
(Visual Studio)
Shanghai をスタートアッププロジェクトに設定
デバッグ > デバッグ開始
```

### 注意
ほぼすべての機能はデフォルトでは無効になっています。
以下のディレクトリにある設定ファイルの編集が必要です。
> ./deploy/settings/

deploy/www/twque/
は CGI から書き込み可能に手動で変更する必要があります。

## 人形語のテスト用インタプリタを実行
```
% ./LangTest/bin/Debug/LangTest.exe
```
```
(Visual Studio)
LangTest をスタートアッププロジェクトに設定
デバッグ > デバッグ開始
```

* 1行入力するごとに別に実行
* 空行入力で終了

## 人形語の仕様書
> DollsLang/spec.MD
