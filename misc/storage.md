# ストレージの選び方

## メインストレージ

メインストレージを選択する際、最も気になるのはもちろん容量だが、
読み書き性能にも同じくらい気を配るべきである
(容量だけで検索すると、色々な性能のが混ざって出てくる)。

* 容量
* スループット
  * ≒シーケンシャルアクセス速度 (R/W)
* レイテンシ
  * ≒ランダムアクセス速度 (R/W)

納得性の高い = すべてにおいて高品質なストレージを選ぼう。

なお、Nintendo Switch 1/2 にも役立つ。

## Micro SD

SD Association: <https://www.sdcard.org/ja/>

従来からのメインストレージ。
なんかパッケージにいっぱいアイコンがついているが、重要。

### SD 容量

規格名が色々あって混乱しそうになるが、容量レンジ (とファイルシステム) を
表しているだけなので、**容量のバイト数だけ見ておけば OK**。

* SD (SD 1.0)
  * .. 2 GB
  * FAT16
  * いわゆる無印時代。組み込み分野としては十分大きいけれど。
* SDHC (High Capacity) (SD 2.0)
  * 2 .. 32 GB
  * FAT32
  * よく使われるようになった時代。
* SDXC (Extended Capacity) (SD 3.0)
  * 32 GB .. 2 TB
  * exFAT
  * 最近のスマホゲーム機 RaspPi のストレージとしてはこのレンジ。
* SDUC (Ultra Capacity) (SD 7.0)
  * 2 TB .. 128 TB
  * exFAT
  * まだ製品は存在しないらしい。
    業務用ビデオカメラ向けだが CFexpress が使われているとか。

### SD バスインタフェース

バスインタフェースはどちらかというとホストコントローラ (= Raspberry Pi) 側の
仕様になる。当然これより速い SD card を入れても速度は出ない。

SDR と DDR は DRAM とかでよくあるアレ。

* SDR = Single Data Rate
* DDR = Double Data Rate
  * クロックの立ち上がりと立ち下がりの両方でデータを転送する。単純計算で速度2倍。

UHS-II から信号線が2本になって送信と受信を同時に行えるようになったらしい。
2本ともを同じ方向に使うことで片方向になる代わりに速度を2倍にすることができ、
これを HD と呼んでいる。
なお回路が複雑になって嫌なので UHS-III では HD はやめたらしい。

* FD = Full Duplex: 全二重通信
* HD = Half Duplex: 半二重通信

<https://www.sdcard.org/ja/developers-2/sd-standard-overview/bus-speed-default-speed-high-speed-uhs-sd-express/>

* SD 1.0
  * Default Speed 12.5 MB/s
    * いわゆる無印。
* SD 1.1
  * High Speed 25 MB/s
* SD 3.0
  * UHS-I (UHS: Ultra High Speed)
    * SDR50 50MB/s
    * DDR50 50MB/s
      * SDR に比べてクロック周波数半分で同じ速度を出すことができる。
    * SDR104 104MB/s
* SD 4.0
  * UHS-II
    * 156MB/s (FD)
    * 312MB/s (HD)
* SD 6.0
  * UHS-III
    * 624MB/s (FD)
* SD 7.0
  * SD Express
    * PCIe Gen.3 x1Lane 985 MB/s
* SD 8.0
  * SD Express
    * 1970 MB/s
      * PCIe Gen.4 × 1Lane
      * PCIe Gen.3 × 2Lane
    * 3940 MB/s
      * PCIe Gen.4 × 2 Lane

Raspberry Pi は UHS-I に対応している。
(3以前は不明)

| Raspberry Pi | Spec |
|-|-|
| 4 | DDR50  |
| 5 | SDR104 |

### SD スピードクラス

スピードクラスは SD card 側のシーケンシャルアクセス性能の最低保証を示す。
必然的にバスインタフェースも保証値に応じたバージョン以上が必要になる。
要はビデオカメラの録画先に使うときにフレーム落ちが発生しない、などを
製品選択の際にアイコンを目印にできるようにするための規格である。

最低保証値であるため、平均や最大速度はこれを大幅に上回ることもある。
R: 200 MB/s のように書かれていることもあるが、これはベンダーからのアピールであり
規格に準拠したアイコンではない。

無印のスピードクラスは大きな C の中に数字が書かれたアイコン。
数字は MB/s を示す。

UHS スピードクラスは大きな U の中に数字が書かれたアイコン。
数字は 10 MB/s を示すので直接速度を表さなくなってしまった。
基本的に C より U の方が速いが、数字が小さく見えてしまう。

ビデオスピードクラスは V の右に数字が書かれたアイコン。
ビデオ向けユースケースの保証値であることを強調したくなったのかもしれない。
数字は MB/s をそのまま示すように戻った。~~お察しである。~~
数値として

* スピードクラス
  * C2: 2 MB/s
  * C4: 4 MB/s
  * C6: 6 MB/s
  * C10: 10 MB/s (High Speed が必要)
* UHS スピードクラス (名前の通り UHS が必要)
  * U1: 10 MB/s (SD 3.01)
  * U3: 30 MB/s (SD 4.00)
* ビデオスピードクラス (SD 5.00)
  * V6: 6 MB/s
  * V10: 10 MB/s
  * V30: 30 MB/s
  * V60: 60 MB/s
  * V90: 90 MB/s

## SSD

Raspberry Pi 5 ではついに PCIe が搭載され、
M.2 SSD からブートすることができるようになった。
~~組み込みって何だよ。~~

SSD HAT
