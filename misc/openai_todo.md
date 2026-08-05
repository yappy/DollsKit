# OpenAI API コード改善候補

調査日: 2026-08-05

対象: `openai.rs`、`openai/` 以下（呼び出し方の確認に必要な範囲で周辺コードも参照）

これは実装前のレビュー結果であり、TODO の全項目を一度に採用すべきという意味ではない。特にモデル選定、会話データの保持、サーバー側 compaction は、品質・費用・プライバシーの要件を決めてから設計する。

## 最優先

### 1. 現行モデルへ移行し、モデル情報の管理方法を見直す

- `MODEL_LIST` は `gpt-4o-mini`、`gpt-4o`、`gpt-4`、`gpt-4-turbo` のみで、現在のモデル選択肢から大きく遅れている（`openai.rs:55-110`）。
- 現在の公式推奨は GPT-5.6 系。代表的な選択肢は、能力優先の `gpt-5.6`（`gpt-5.6-sol` への alias）、費用とのバランスを取る `gpt-5.6-terra`、高頻度・費用優先の `gpt-5.6-luna`。この用途では現在の既定値が `gpt-4o-mini` なので、まず `terra` と `luna` を実データで比較するのが妥当そう。
- GPT-5.6 系は context window 1,050,000、最大入力 922,000、最大出力 128,000。現行の `context_window` と `max_output_tokens` だけでは「最大入力」と「入力 + 出力 + reasoning の総枠」を正確に表現できないため、`max_input_tokens` も別フィールドにする。
- `model_name: &'static str` と固定表からの検索により、設定ファイルに未登録の新モデルや snapshot を書けない。所有する `String` にし、モデル ID と能力情報を分離する。alias（追随重視）と snapshot（再現性重視）を設定で選べるようにする。
- `/v1/models/{model}` から得ているのは作成日時などで、context/output limit は得られない。24 時間キャッシュしている `OnlineModelInfo` は制限管理には役立っていないため、用途を明確化するか削除を検討する。
- GPT-5.6 は reasoning model なので、`reasoning.effort`（まず `low` または `medium` を比較）と、必要なら `text.verbosity` を `ResponseRequest` で指定可能にする。`temperature` / `top_p` を全モデル共通の中心設定とみなさない。

公式資料: [最新モデルの選択](https://developers.openai.com/api/docs/guides/latest-model)、[GPT-5.6 Sol](https://developers.openai.com/api/docs/models/gpt-5.6)、[GPT-5.6 Terra](https://developers.openai.com/api/docs/models/gpt-5.6-terra)、[GPT-5.6 Luna](https://developers.openai.com/api/docs/models/gpt-5.6-luna)

### 2. `ChatHistory` の手動概算を、API に基づく管理へ置き換える

- 現在は本文文字列だけを `tiktoken_rs` で数えている（`openai/chat_history.rs:39-49`, `60-175`）。メッセージの構造、instructions、tools の JSON Schema、role、call ID、reasoning item などが計上されない。
- function 定義を一律 800 tokens とする `FUNCTION_TOKEN`（`openai/function.rs:17-20`）も、tool の個数・description・schema サイズに連動しない。
- low-detail 画像を常に 85 tokens とする値（`openai/chat_history.rs:92-106`）は `gpt-4o` 等の一部モデル向けであり、モデルによっては 85 ではない。現行 GPT-5.6 の画像計算も同じ前提ではない。
- 新しいモデルを `tiktoken_rs::bpe_for_model()` / `get_context_size()` がまだ知らない場合、`unwrap()` で起動時に panic する。未知モデルをエラーとして扱うか、fallback encoding と明示的なモデル制限を使う。
- 公式の `POST /v1/responses/input_tokens` は、実際の Responses 入力に対する `input_tokens` を返す。送信前に厳密な判定が必要な箇所ではこれを使えば、固定 800 や画像 85、個別の手計算をまとめて廃止できる。
- 毎ターン token-count API を呼ぶと latency と API 呼び出しが増える。候補設計は次のいずれか。
  1. 通常は Responses の server-side compaction / `truncation: auto` に任せ、ローカル計数を持たない。
  2. ローカル計数は UI 表示用の概算だけにし、上限付近でのみ input-token count API を呼ぶ。
  3. 厳密な事前保証が必要なら毎回 count API を呼び、その同じ request body を Responses に送る。
- 成功後は `ResponseObject.usage` の `input_tokens`、`output_tokens`、`reasoning_tokens`、`cached_tokens` を記録し、概算との差ではなく実測値で費用・履歴方針を調整する。GPT-5.6 では `cache_write_tokens` も追加で扱う。

公式資料: [Token counting](https://developers.openai.com/api/docs/guides/token-counting)、[画像入力の token 計算](https://developers.openai.com/api/docs/guides/images-vision#calculating-costs)、[Responses API の usage](https://developers.openai.com/api/reference/resources/responses)

### 3. Responses API の会話状態を正しく保持する

- 現在は assistant の表示テキスト、web search call、function call/output だけを独自の `InputItem` に詰め直している（`openai/chat_history.rs:113-175`）。GPT-5 系の reasoning item、暗号化 reasoning、message の `phase`、compaction item、refusal/annotation 等が失われる。
- 公式ガイドは stateless 運用なら `response.output` の全 item をそのまま次の input に追加するよう要求している。特に reasoning model で一部だけ再構築すると品質低下や会話継続の不整合につながる。
- 最も簡潔なのは `previous_response_id` で連結し、各ターンでは新規入力だけ送る方法。ただし response は既定で保存され、過去の input tokens も毎ターン課金対象になる。30 日保存が要件に合うかを先に確認する。
- 保存を避けるなら `store: false` とし、返された全 output item を lossless に保持・再送する。長時間会話では `context_management: [{type: "compaction", compact_threshold: ...}]` による server-side compaction、または `/v1/responses/compact` を使う。
- 単純に古い `VecDeque` 要素を削る現在の方式は、tool call と output の対応や重要な初期条件を壊し得る。compaction を採用できない場合でも、turn/tool-call 単位の整合性と固定 developer instruction を保つ設計にする。

公式資料: [Conversation state](https://developers.openai.com/api/docs/guides/conversation-state)、[Compaction](https://developers.openai.com/api/docs/guides/compaction)

### 4. Responses の型定義を現行 schema に追随させ、未知 item で全体が失敗しないようにする

- `OutputElement` は `message`、`function_call`、`web_search_call` の 3 種しか受けない（`openai.rs:668-721`）。GPT-5 系で通常現れ得る `reasoning` を含め、file search、image generation、compaction など別の output item が一つあるだけで response 全体の deserialize が失敗する可能性が高い。
- `InputItem` も同様に対応型が少なく、公式が推奨する「全 output item の再送」を表現できない（`openai.rs:307-359`）。
- 少なくとも利用する item 型を追加し、未対応型は raw `serde_json::Value` として lossless に保持できる設計にする。出力表示用の typed view と、会話継続用の raw item を分離すると追随しやすい。
- 手書き schema の drift が既に複数箇所にあるため、OpenAPI schema からの生成、または request/response の境界だけを薄い DTO として保ち raw JSON を併用する方法を検討する。Rust の非公式 SDK を無条件に導入するより、更新責任と依存リスクを比較する。
- response の `status`、`incomplete_details`、各 output item の status、拒否、annotation/citation を扱い、「HTTP 200 かつ空文字列」を成功扱いしない。

公式資料: [Responses create API](https://developers.openai.com/api/reference/resources/responses/methods/create)、[Conversation state](https://developers.openai.com/api/docs/guides/conversation-state)

## 高優先度

### 5. 出力予約を「実際の request 上限」にする

- `get_output_reserved_token()` は最大出力の 105% と context の 20% の小さい方を履歴から引くが（`openai.rs:1025-1034`）、実際の request では `max_output_tokens` を設定していない（`openai.rs:1184-1215`）。ローカルで予約した量と API が生成可能な量が一致しない。
- reasoning tokens も `max_output_tokens` に含まれるため、表示テキスト用の余白と reasoning 用の余白を同一視しない。用途ごとの明示的な output budget を request に渡し、その値を入力許容量の計算にも使う。
- 現行モデルでは context window、max input、max output を分け、`input <= max_input` かつ `input + output/reasoning <= context` の双方を満たすようにする。105% の安全率で最大値より多く予約する方式はやめ、設定した上限と safety margin を明示する。

### 6. web search tool 名と結果/citation の扱いを更新する

- 現在は `type: "web_search_preview"` を生成する `WebSearchPreview`（`openai.rs:395-407`）だが、現行 Responses API の例は `type: "web_search"`。
- 現在の output parser は search call の `id` と `status` しか保持せず、回答テキストの URL citation annotation や source 情報を落としている。ユーザーへ検索結果を返す用途なら annotation/source を保持し、表示側で citation を関連付ける。
- `include: ["web_search_call.action.sources"]`、検索コンテキスト量、地域設定等は必要な場合だけ request option として公開する。

公式資料: [Using tools](https://developers.openai.com/api/docs/guides/tools)、[Web search](https://developers.openai.com/api/docs/guides/tools-web-search)

### 7. Image API を GPT Image 2 に対応させる

- `ImageGenRequest` に `model` がなく、旧 DALL-E 前提の `256x256` と URL response を想定している（`openai.rs:754-818`, `1264-1285`）。現行の最新 Image API model は `gpt-image-2` で、公式例は `b64_json` を受け取る。
- `model` を必須または設定可能にし、GPT Image の現行 `size`、`quality`、`output_format`、compression、background 等に型を更新する。戻り値も URL の `Vec<String>` ではなく、画像 bytes + MIME/format 等を表す型が自然。
- 単発生成なら Image API のままでよい。会話内での修正・反復生成が必要なら Responses API の `image_generation` tool に統合すると `previous_response_id` で多 turn 編集できる。
- prompt 上限などのコメントが旧仕様のままなので、モデル固有制約をコード内の固定コメント/定数にしすぎない。

公式資料: [Image generation](https://developers.openai.com/api/docs/guides/image-generation)

### 8. TTS を `gpt-4o-mini-tts` と現行 voice に対応させる

- `SpeechModel` は `tts-1` / `tts-1-hd` のみ、voice は 6 種のみ（`openai.rs:823-879`）。現行の推奨 TTS model は `gpt-4o-mini-tts` で、`marin`、`cedar` 等を含む voice が追加されている。
- `instructions` を request に追加すると話し方、感情、アクセント等を指定できる。モデル別の voice 対応差を検証する。
- `input.len()` は UTF-8 byte 数であり、「characters」判定ではない（`openai.rs:1299-1302`）。日本語を不必要に短く制限する。API の現行制限に合わせ、必要なら `chars().count()` または token/API error ベースにする。
- streaming response を使えば再生開始を早められる。利用者には AI 音声であることを明示する要件も UI/呼び出し側で確認する。

公式資料: [Text to speech](https://developers.openai.com/api/docs/guides/text-to-speech)

## 中優先度

### 9. request option を追加し、deprecated な `user` を置き換える

- `ResponseRequest` は現行 API の主要 option を多く表現できない（`openai.rs:538-614`）。少なくとも `store`、`reasoning`、`text.verbosity`、`truncation` / `context_management`、`parallel_tool_calls`、`tool_choice`、`safety_identifier`、`prompt_cache_key` を用途に応じて追加する。
- `user` は現行 API では置き換え対象。安全対策用には個人情報を含まない安定した `safety_identifier`、cache routing には別の `prompt_cache_key` を使う。
- tools が空の場合は `Some(vec![])` ではなく省略して request を小さくする。

### 10. Prompt Caching を実測して活用する

- instructions と tool schema のような静的 prefix を先頭に固定し、可変の会話内容を後ろにする。`usage.input_tokens_details.cached_tokens` をログ/metrics に出す。
- GPT-5.6 では cache write tokens も課金されるため、単に cache を有効にすれば安くなるとは限らない。反復率が高い固定 prefix だけに explicit breakpoint を置く案を評価する。
- tool schema を毎回同じ順序で serialize する。現在 properties が `HashMap` なので順序が実行ごとに変わり得る。`BTreeMap` または安定 sort にすると exact prefix match と再現性に有利。

公式資料: [Prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching)

### 11. timeout、retry、error 分類を endpoint ごとに見直す

- 全 endpoint 共通 60 秒 timeout は reasoning、web search、画像生成、TTS には短い場合がある（`openai.rs:30-32`）。endpoint/request 種別ごとの timeout、Responses streaming、必要なら background mode を検討する。
- 汎用 `send_with_retry` が POST をどの条件で再送するか確認する。画像生成などは再試行で課金や重複処理が起き得るため、接続失敗・429・5xx の区別、最大 backoff、server hint を明確にする。
- 429 の本文に `rate`/`limit` や `quota`/`billing` が含まれるかで分類する方式（`openai.rs:1048-1087`）は脆い。OpenAI の構造化 error object の `type` / `code` を parse して分類し、request ID を error に添える。
- rate-limit header が一つ欠けただけで全情報を捨てず、各値を `Option` で読む。reset まで線形回復すると仮定した `calc_expected_current()` は実際のサーバー残量を保証しないので、「推測値」であることを API 名/表示にも明確にする。

### 12. request/response 本文のログを安全にする

- 現在は request body 全体と response JSON 全体を info log に出す（`openai.rs:1112-1114`, `1157-1159`）。会話本文、function output、個人情報、base64 画像がログへ残り、容量も急増する。
- info では endpoint、model、request ID、status、latency、usage、item 種別/個数だけにする。本文は明示的な debug opt-in、redaction、文字数上限付きにする。画像 data URL は既に `Debug` で長さだけにしているが、他の文字列も同様に扱う。

### 13. 画像入力の品質と検証を改善する

- 常に 512px、PNG、low detail、Nearest 補間へ変換している（`openai.rs:1218-1261`）。文字や細部が必要な画像では品質が大きく落ちる。
- 用途別に `low` / `high` / `auto`（GPT-5.6 では必要なら `original`）を選べるようにし、縮小には Lanczos 系を検討する。写真を常に PNG にすると送信量が増えるので、元形式または品質指定 JPEG/WebP も候補。
- byte size、pixel 数、画像枚数をローカルで検証し、token 数はモデル別固定値ではなく count API または usage に任せる。

## 低優先度・保守性

### 14. function schema の表現力と検証を整理する

- `strict: true`、全 property を required、optional は `null` union、`additionalProperties: false` という現在の基本方針は公式推奨と合っている。
- 一方、`ParameterElement` が primitive と string enum だけなので、配列、入れ子 object、数値範囲等を表せない。必要になった時点で JSON Schema をより一般的に表せる型、または `serde_json::Value` / schema builder へ移す。
- `required` と `properties` の不一致、strict schema 制約を登録時に検証するテストを追加する。現在 optional に見えるフィールドを required + null にしている箇所は意図をコメントで統一する。

公式資料: [Function calling の strict mode](https://developers.openai.com/api/docs/guides/function-calling#strict-mode)

### 15. API contract test と fixture を追加する

- live API を使わない JSON fixture test を用意し、reasoning、function call、web search citation、refusal、incomplete response、未知 output item、usage details を deserialize できることを確認する。
- request の golden test で model、tools、strict schema、`store`、reasoning/output budget が意図どおり serialize されることを確認する。
- live smoke test は課金・network が必要なので `#[ignore]` のまま分離し、対象 model を環境変数/設定から渡す。固定 `gpt-4o` の tokenizer test は、新方式に合わせて exact API count test とローカル概算 test を分ける。

## 推奨する実装順

1. response/input item を lossless に扱えるようにして GPT-5 reasoning output に対応する。
2. model 設定を固定 `&'static str` から切り離し、`gpt-5.6-terra` / `luna` を評価できるようにする。
3. `max_output_tokens` と reasoning 設定を request に実際に反映する。
4. 会話状態を `previous_response_id` または `store:false` + 全 output 再送のどちらかへ統一する。
5. server-side compaction と input-token count API を導入し、固定 `FUNCTION_TOKEN`、画像 85、手動 context size を削る。
6. web search、Image API、TTS の各 schema を現行仕様へ更新する。
7. usage/cache metrics、ログ redaction、error/retry/timeout を整備する。

## 設計時に決める必要がある事項

- 品質・価格・latency のどれを優先するか（`sol` / `terra` / `luna`、reasoning effort）。
- response を OpenAI 側に保存して `previous_response_id` を使ってよいか。保存不可なら stateless + encrypted reasoning/compaction item 保持が必要。
- 履歴切り捨て時に、古い内容を単に忘れてよいか、compaction で要約状態を残すべきか。
- 画像生成の戻り値を現在の URL として維持する必要があるか、base64/bytes を扱える呼び出し側変更が可能か。
- token count の正確性と追加 API latency のどちらを優先するか。
