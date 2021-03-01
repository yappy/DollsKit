# 技術ノート

## sleepy-discord voice 所有権問題

Client (継承してイベントハンドラをオーバーライドして使うクラス) は以下のリストを持つ
* VoiceContext
  * unique_ptr to EventHandler
    * Voice chat ready, start speaking, finish speaking 等のハンドラインタフェース
* VoiceConnection
  * unique_ptr to AudioSource
    * data read インタフェース

この2つは切断時に非同期にコンテナから削除される可能性がある。
この時 unique_ptr のデストラクタから delete される。

## AudioSource の read()

ファイル終端などで半端なリードサイズを返すとクラッシュする。
無音で埋めるか最後の方のデータは捨てるかして対応する。
