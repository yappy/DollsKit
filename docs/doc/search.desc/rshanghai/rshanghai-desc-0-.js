searchState.loadedDescShard("rshanghai", 0, "Rust 版管理人形。\nログフィルタのためのクレート名。…\nCron 設定例の出力先。\nCron 用シェルスクリプトの出力先。\nCron 用シェルスクリプトの出力先。\nログのファイル出力先。\nデーモン化の際に指定する pid …\nデーモン化の際に指定する stderr …\nデーモン化の際に指定する stdout …\n…\n…\n実行可能パーミッション 755 …\nstdout, stderr …\nロギングシステムを有効化する。\nエントリポイント。\nコマンドラインのヘルプを表示する。\n基本的なシステム関連。\nシステムモジュール関連。\nシステムメイン処理。 …\n各種ユーティリティ。\n設定データの管理。\n非同期タスクを管理する。\nバージョン情報。\n設定データ(グローバル変数)。\n現在設定の出力パス。\nデフォルト設定の出力パス。\nロードする設定ファイルパス。\n設定データ。\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\n設定データをロードする。\nシステムシャットダウン開始通知受信側 …\nシステムシャットダウン開始通知送信側 …\nTaskServer …\nTaskServer::run の返す実行終了種別。\nタスクサーバ本体。\nシステムシャットダウン時、true …\n…\n各タスクに clone して渡すオリジナルの …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nタスクサーバを生成して初期化する。\nTokio ランタイム。\n…\n1回限りのタスクを生成して実行開始する。\nspawn_oneshot_task を内蔵の Self::ctrl …\n周期タスクを生成する。\ncrate::sysmod::SystemModule …\n全システムモジュールのリスト。\nキャンセル通知を待つ。\nビルドプロファイルを “debug” または “…\nrustc コンパイラバージョン “major.minor.patch…\n…\n…\nSystemModules 内の SystemModule …\nシステムモジュールが実装するトレイト。\n…\nRaspberry Pi カメラ機能。\nDiscord クライアント (bot) 機能。\n全 SystemModule …\nReturns the argument unchanged.\n定期ヘルスチェック機能。\nHTTP Server 機能。\nCalls <code>U::from(self)</code>.\nLINE API。\nSystemModule の初期化時には …\nOpenAI API.\nシステム情報。\nTwitter 機能。\nCamera システムモジュール。\nカメラ設定データ。toml 設定に対応する。\n縦デフォルトサイズ。\njpeg デフォルトクオリティ。\nデフォルト撮影時間(ms)。TO …\n横デフォルトサイズ。\n縦最大サイズ。(Raspberry Pi Camera V2)\njpeg 最大クオリティ。\n横最大サイズ。(Raspberry Pi Camera V2)\n縦最小サイズ。(Raspberry Pi Camera V2)\njpeg 最小クオリティ。\n横最小サイズ。(Raspberry Pi Camera V2)\n画像リストは BTreeMap …\nストレージ上の画像を示すエントリ。\n…\nサムネイルの縦サイズ。\nサムネイルファイル名のポストフィクス。\nサムネイルの横サイズ。\n写真撮影オプション。\n自動撮影タスク。\nPicDict の合計ファイルサイズを計算する。\n必要に応じて自動削除を行う。\n設定データ。\nキーからファイル名を生成する。\nサムネイルを作成する。 成功すれば jpeg …\n…\nカメラ自動撮影タスクを有効化する。\nraspistill …\ninit_pics 用の再帰関数。\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\n縦サイズ。\n検索ルートディレクトリ内から jpg …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nコンストラクタ。\nasync 使用可能になってからの初期化。\n画像一覧ページの1ページ当たりの画像数。\nメイン画像のファイルパス。\nサムネイル画像のファイルパス。\nSelf::pic_history_dir …\nSelf::pic_archive_list …\n撮影した画像を保存するディレクトリ。 …\n撮影された画像リスト。自動削除対象。\nストレージ上の画像リスト (history, archive) …\nヒストリ内の <code>key</code> …\n…\njpeg クオリティ。\n画像をリサイズする。 成功すれば jpeg …\nストレージ上の画像リストデータ。\n写真を撮影する。成功すると jpeg …\n撮影時間(ms)。\nSelf::path_main と Self::path_th …\nSelf::pic_history_dir …\n横サイズ。\n自動撮影の時刻リスト。\nApplication command context\n…\nDiscordPrompt のデフォルト値。\nDiscord システムモジュール。\nDiscord 設定データ。toml 設定に対応する。\nメッセージの最大文字数。 (Unicode codepoint)\nPrefix command context\n自動削除機能の対象とするチャネル ID …\n自動削除機能の設定データ。\nai コマンドの会話履歴。\nSelf::chat_history の有効期限。\nSelf::chat_history にタイムアウトを適用する。\n設定データ。\n分を (日, 時間, 分) に変換する。\n現在有効な Discord Client コンテキスト。\n…\nシステムを初期化し開始する。\n…\n機能を有効化するなら true。\nSerenity の全イベントハンドラ。\nto_string 可能にする。\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nOpenAI function 機能テーブル\n会話履歴をクリアするまでの時間。\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\n残す数。0 は無効。\n残す時間 (単位は分)。0 は無効。\nコンストラクタ。\nメッセージの発言先チャネル。 Discord …\nPoise イベントハンドラ。\nasync 使用可能になってからの初期化。\nオーナーのユーザ ID。 Discord bot …\n日時分からなる文字列を分に変換する。\n自動削除周期タスク。\nパーミッションエラーメッセージ。 …\nSelf::ctx が None …\n…\nPoise イベントハンドラ。\nOpenAI プロンプト。\n…\nMarkdown エスケープしながら Markdown …\n発言を投稿する。\nアクセストークン。Developer Portal …\n定期実行の時刻リスト。\n1: Arm frequency capped\nCPU 情報。\nディスク使用率。\nHealth::history の最大サイズ。\nヘルスチェックシステムモジュール。\nヘルスチェック設定データ。toml …\n履歴データのエントリ。\nメモリ使用率。\n17: Arm frequency capping has occurred\n19: Soft temperature limit has occurred\n18: Throttling has occurred\n16: Under-voltage has occurred\n3: Soft temperature limit active\n2: Currently throttled\nvcgencmd get_throttled bit flags\n0: Under-voltage detected\nGet a flags value with all known bits set.\n利用可能ディスクサイズ (GiB)。\n利用可能メモリ量 (MiB)。\nThe bitwise and (<code>&amp;</code>) of the bits in two flags values.\nThe bitwise and (<code>&amp;</code>) of the bits in two flags values.\nThe bitwise or (<code>|</code>) of the bits in two flags values.\nThe bitwise or (<code>|</code>) of the bits in two flags values.\nGet the underlying bits value.\nThe bitwise exclusive-or (<code>^</code>) of the bits in two flags …\nThe bitwise exclusive-or (<code>^</code>) of the bits in two flags …\n測定タスク。 Self::history …\nSelf::check_task のエントリ関数。 …\nThe bitwise negation (<code>!</code>) of the bits in a flags value, …\n設定データ。\nWhether all set bits in a source flags value are also set …\nCPU 使用率。\n全コア合計の使用率。\n…\nThe intersection of a source flags value with the …\nディスク使用率。\nGet a flags value with all bits unset.\nヘルスチェック機能を有効化する。\nThe bitwise or (<code>|</code>) of the bits in each flags value.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nConvert from a bits value.\nConvert from a bits value exactly.\nConvert from a bits value, unsetting any unknown bits.\nThe bitwise or (<code>|</code>) of the bits in each flags value.\nGet a flags value with the bits of a flag with the given …\nCPU 論理コア数を取得する。\nCpuInfo を計測する。\nCPU 温度 …\nCPU クロック周波数を取得する。\nDiskInfo を計測する。\nCPU クロック周波数の設定値を取得する。 …\nMemInfo を計測する。\nCPU スロットリング状態を取得する。\n測定データの履歴。最大サイズは …\nThe bitwise or (<code>|</code>) of the bits in two flags values.\nThe bitwise and (<code>&amp;</code>) of the bits in two flags values.\nWhether any set bits in a source flags value are also set …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nWhether all known bits in this flags value are set.\nWhether all bits in this flags value are unset.\nYield a set of contained flags values.\nYield a set of contained named flags values.\nメモリ使用率。\nコンストラクタ。\nThe bitwise negation (<code>!</code>) of the bits in a flags value, …\nThe intersection of a source flags value with the …\nCall <code>insert</code> when <code>value</code> is <code>true</code> or <code>remove</code> when <code>value</code> is …\nThe intersection of a source flags value with the …\nThe intersection of a source flags value with the …\nThe bitwise exclusive-or (<code>^</code>) of the bits in two flags …\nCPU 温度 (℃)。 取得できなかった場合は None…\nタイムスタンプ。\nThe bitwise exclusive-or (<code>^</code>) of the bits in two flags …\nディスク総量 (GiB)。\nメモリ総量 (MiB)。\nツイートタスク。 Self::history …\nSelf::tweet_task のエントリ関数。 …\nThe bitwise or (<code>|</code>) of the bits in two flags values.\n定期実行の時刻リスト。\n定期実行の時刻リスト。\nContains the error value\nHTTP Server 設定データ。toml 設定に対応する。\nContains the success value\nHTTP Server 機能を有効化する。\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGitHub Hook 機能を有効化する。パスは /<em>rootpath</em>…\nGitHub Hook の SHA256 …\nGithub Webhook.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nLINE Webhook.\nLINE webhook 機能を有効化する。パスは /<em>rootpath</em>…\n…\nポート番号。\n管理者専用ページを有効化する。\n…\nUploader.\nアップロードされたファイルの保存場所。\nアップローダ機能を有効化する。パスは /…\nReturns the argument unchanged.\nReturns the argument unchanged.\nGithub webhook 設定で Content type = application/json …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\ntype = “user”\n“active” or “standby”\n署名検証後の POST request 処理本体。\n署名検証。\nGET /priv/camera/archive/ …\nPOST /priv/camera/archive\nhistory/archive 共用写真リスト HTML 生成。\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGET /priv/camera/history/ 写真一覧。\nPOST /priv/camera/history\nGET /priv/camera/ Camera インデックスページ。\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\n…\nGET /priv/camera/pic/history/{name}/{kind} …\nGET /priv/camera/pic/history/{name}/{kind} …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\n…\nファイル名をチェックする。\nMultipartError に Send …\nReturns the argument unchanged.\nReturns the argument unchanged.\n…\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nLinePrompt のデフォルト値。\nLINE システムモジュール。\nDiscord 設定データ。toml 設定に対応する。\nMessage::Text の最大文字数。 mention …\nLINE API タイムアウト。\nチャネルシークレット。\nai コマンドの会話履歴。\nSelf::chat_history の有効期限。\nSelf::chat_history にタイムアウトを適用する。\nレスポンスの内容を確認しながら json …\nHTTP クライアント。\n設定データ。\n…\n機能を有効化するなら true。\nOpenAI API エラー時のメッセージ。\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nOpenAI function 機能テーブル\n会話履歴をクリアするまでの時間。\nLINE ID から名前への固定マップ。\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nlen = 1..=5\nlen = 1..=5\nコンストラクタ。\n…\n…\n…\n…\nuserId, groupIdm or roomId\nOpenAI API タイムアウト時のメッセージ。\nアクセストークン。Developer Portal …\nurl len &lt;= 5000 protocol = https (&gt;= TLS 1.2) format = …\nurl len &lt;= 5000 protocol = https (&gt;= TLS 1.2) format = …\nHTTP 通信のタイムアウト。 …\nOpenAI API JSON 定義。 会話メッセージ。\nOpenAI API JSON 定義。 会話リクエスト。\nOpenAI API JSON 定義。 会話応答データ。\nOpenAI API JSON 定義。 応答案。\nOpenAI API JSON 定義。 function 定義。\nOpenAI API JSON 定義。 function 呼び出し。\nOpenAI API JSON 定義。 画像データ。\nOpenAI API JSON 定義。 画像生成リクエスト。\nOpenAI API JSON 定義。 画像生成レスポンス。\nOpenAI API JSON 定義。 画像生成のサイズ。\nモデル情報。一番上がデフォルト。\nモデル情報。\n…\nOpenAI システムモジュール。\nOpenAI 設定データ。\nOpenAI API JSON 定義。 function パラメータ定義。\nOpenAI API JSON 定義。 function パラメータ定義。\nOpenAI API JSON 定義。 …\nOpenAI API JSON 定義。 ChatMessage …\nOpenAI API JSON 定義。 音声生成リクエスト。\nAI 応答待ちのタイムアウト。\nThe latest text to speech model, optimized for speed.\nThe latest text to speech model, optimized for quality.\nhttps://platform.openai.com/docs/api-reference/chat/create\nOpenAI API JSON 定義。 トークン消費量。\nOpenAI API のキー。\nOpenAI API - function 機能向け、 …\nOpenAI Chat API を使用する。\nOpenAI Chat API を fcuntion call …\nRequired even if None (null)\nOpenAI API 利用を有効にする。\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nOpenAI API - function.\nOpenAI Image Generation API を使用する。\nMODEL_LIST からモデル名で ModelInfo …\n出力用に予約するトークン数を計算する。\nThe text to generate audio for. The maximum length is 4096 …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nエラーチェインの中から reqwest …\nOne of the available TTS models: tts-1 or tts-1-hd\n使用するモデル名。 MODEL_LIST から選択。\nThe number of images to generate. Must be between 1 and 10.\nRequired if role is “function”\nコンストラクタ。\nA text description of the desired image(s). The maximum …\nThe format in which the generated images are returned. …\nThe format to audio in. Supported formats are mp3, opus, …\n“system”, “user”, “assistant”, or “function…\nThe size of the generated images. Must be one of 256x256, …\nThe speed of the generated audio. Select a value from 0.25 …\nOpenAI Create Speech API を使用する。\n…\ne.g. “string”\n“object”\nA unique identifier representing your end-user, which can …\nThe voice to use when generating the audio. Supported …\n計算関連。\nシステム情報取得。\nこのモジュール以下の全ての関数を …\nシステム情報取得。\nWeb アクセス関連。\n数式を計算する。\nこのモジュールの関数をすべて登録する。\nダイスまたはコイン数の最大値。\nダイスまたはコイン数の最小値。\nダイスの面数の最大値。\nダイスの面数の最小値。\nサイコロを振る。\nこのモジュールの関数をすべて登録する。\nサイコロを振る。\nReturns the argument unchanged.\nCPU 使用率情報取得。\n現在の日時を取得する。\nモデル情報取得。\nバージョン情報取得。\nCalls <code>U::from(self)</code>.\nこのモジュールの関数をすべて登録する。\nHTML …\n気象情報を取得する。\nこのモジュールの関数をすべて登録する。\nURL に対して GET …\n引数は JSON ソース文字列で与えられる。 …\n…\nFunction …\n引数。文字列から Json value へのマップ。\n関数の Rust 上での定義。\nsync fn で、async fn …\nOpenAI API json 定義の再エクスポート。\nOpenAI function 群の管理テーブル。\n関数を呼び出す。\nSelf::call の内部メイン処理。\n関数名から Rust 関数へのマップ。\n関数一覧のヘルプ文字列を生成する。\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nOpenAI API に渡すためのリストを取得する。\nOpenAI API に渡すためのリスト。\nargs から引数名で i64 を取得する。 …\nargs から引数名で文字列値を取得する。 …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nbasicfuncs …\n関数を登録する。\nシステム情報構造体。\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\n起動時間。\nTwitterPrompt のデフォルト値。\nTimelineCheck のデフォルト値。\nHTTP header や query を表すデータ構造。\nTwitter 応答設定データ。\nTwitter 応答設定データの要素。\nTwitter 設定データ。toml 設定に対応する。\nOpenAI プロンプト設定。\nTwitter API のアカウント情報。\nTwitter API のアカウント情報。\nOpenAI API 応答を起動するハッシュタグ。\nTwitter API のアカウント情報。\nTwitter API のアカウント情報。\n全 AI リプライを生成する\nHTTP header に設定する (key, value) …\nOAuth 1.0a 認証のための KeyValue …\n全リプライを生成する\nHMAC-SHA1 署名を計算する。 この結果を …\n…\ntweet.fields=entities\n…\n長文ツイートの画像化に使う ttf …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\n自身の Twitter ID を返す。 Self::users_me の …\n自分のツイートリストを得て最終ツイート …\nID -&gt; screen name のマップ。\nexpansions=author_id\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\n添付メディアデータ。\n…\n自身の User …\nSelf::result_count = 0 だと存在しない\nSelf::result_count = 0 だと存在しない\ntext から pat を検索する。 先頭が ‘^’ …\nマッチパターンと応答のリスト。\nOpenAI プロンプト。\nuser name (screen name) から id を取得する。 id -&gt; …\nドキュメントには count …\nタイムラインチェックのルール。…\n本文。media.media_ids が無いなら必須。\nタイムラインチェックの際の走査開始 tweet …\nタイムラインの定期確認を有効にする。\n140 字に切り詰める\nシンプルなツイート。 中身は Self::tweet_raw。\nメディア付きツイート。 中身は Self::tweet_raw…\nTwitterConfig::fake_tweet …\nTwitter 巡回タスク。\nエントリ関数。Self::twitter_task を呼ぶ。\n対象とするユーザ名 (Screen Name) のリスト。\nscreen name -&gt; User オブジェクトのマップ。\nOpenAI API …\nURL encoding や SHA 計算等のユーティリティ。\n字句解析・構文解析関連。\n遊びの道具。\n気象情報。 気象庁の非公式 API …\n会話履歴管理。\n履歴データ。\n全履歴をクリアする。\nトークナイザ。\nトークン列から文字列に復元する。\nReturns the argument unchanged.\nReturns the argument unchanged.\nトークン制限総量を返す。\n履歴データのキュー。\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\n全履歴を走査するイテレータを返す。\n履歴の数を返す。\nメッセージ。\nコンストラクタ。\nヒストリの最後にエントリを追加する。\nトークン数合計上限を減らす。\n文章のトークン数を数える。\n現在のトークン数合計。\nSelf::msg のトークン数。\nトークン数合計上限。\n文章をトークン化する。\nトークン数。\n現在のトークン数使用量を (usage / total) …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\npercent_encode で変換する文字セット。\nHTTP status が成功 (200 台) でなければ Err …\nHTTP status が成功 (200 台) でなければ Err …\ncheck_http_resp 付きの GET。\n文字列を JSON としてパースし、T …\nReturns the argument unchanged.\nHMAC SHA1 を計算する。\nHMAC SHA2 を計算して検証する。\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\n次の1文字を取得し消費する。\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\n次のトークンを取得する。None は EOF …\nexpr = term\nfactor = unary  (&lt;<em>&gt;|&lt;/&gt;|&lt;%&gt; unary)</em>\nexpr = expr &lt;EOF&gt;\nprimary = &lt;(&gt; expr &lt;)&gt; | &lt;INTEGER&gt;\nterm = factor ((&lt;+&gt;|&lt;-&gt; factor)*\nunary = (&lt;+&gt;|&lt;-&gt;)* primary\n次の1文字を取得するが消費しない。\n乱数生成によるダイスロール。\nダイスの個数の最大値。\nダイスの面数の最大値。\nダイスを振る。\n明日から7日間\nhttps://www.jma.go.jp/bosai/common/const/area.json\nhttps://www.jma.go.jp/bosai/forecast/\n今日から6時間ごと、5回分\n明日の 0:00+9:00 と 9:00+9:00 …\n明日から7日間\nJSON 中には存在しない。後でキーを入れる。\nDateStr =&gt; DateDataElem\n…\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\noffice_code から forecast URL を得る。\noffice_code から overview_forecast URL を得る。\nAI にも読みやすい JSON に整形する。")