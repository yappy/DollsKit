# OpenAI API

正式名称が分かりやすいところに無く、よく分からない。
多分 OpenAI API。

ドキュメントは "Docs" と "API Reference" の2種類があり、
似たような内容で違うことが書いてある。

<https://platform.openai.com/docs/> \
<https://platform.openai.com/docs/api-reference/introduction>

* API を使用するプログラムを書ける人のみが対象。
  * 公式ライブラリはあるがそれ以上の甘えは不可能。
  * 本命はこれをバックエンドにサービスを展開してくれる人と思われる。
* 従量課金
  * 初めての人にはお試しで無料クレジットがプレゼントされる。多分今も。
  * オートチャージを設定しなければ無茶な金額を請求されたりはしない。
  * 最新のフラッグシップモデルを選んだりしなければ、個人使用なら 10 ドルも課金すれば
    長く遊べる。多分。画像や音声生成とかは怪しいかもしれない。

Chat GPT はこちら。
ドメインから別サービスとして展開されているが、会社 (Open AI)および
内部で使っているモデルは一緒。
(ただし "現在 ChatGPT で使われているのと同一モデル" はちょっと高い気がする。)

<https://chatgpt.com/>

* 一般人向け
* 無料プラン、または月に一回の定額サブスクライブ制。
* 中身 (モデル) は一緒なので、プログラムを書く前にどんな感じのことができそうか
  試すのに便利。新機能の確認に最適。

## 登録

忘れた。。
Chat GPT と同じアカウントを使えたかもよく覚えていない。

## モデル名

<https://platform.openai.com/docs/models> \
<https://platform.openai.com/docs/pricing>

* "gpt-4o" のような一般名は、"gpt-4o-2024-08-06" のような日付指定のバージョンへの
  リンクになっており、リンク先は自動で無難な新しいバージョンに切り替わる。
  * いつの間にか性格が変わったように感じるかもしれない？
* それが嫌な場合は日時付きのバージョンを指定すれば固定できる。
  * ただし古いものは割高な価格設定になっている気がする。
    メンテナンスコストとかあるからね…。
* 本当にすごい奴は、本当にすごい値段で売られている。
  性能は本当にすごいらしいので、そこは個人の価値観次第。

## エンドポイントの例

## 認証

## 入力フォーマット

<https://platform.openai.com/docs/api-reference/responses/create>

分かりづらくてきれそう。
Playground で function 実行して、うまくいった実績のある実行サンプルから解読する。

```json
// "input":
[
  {
    "role": "user",
    "content": [
      {
        "type": "input_text",
        "text": "東京"
      }
    ]
  },
  {
    "type": "function_call",
    "call_id": "call_UbxMBIKVNhA8iVxWj7Qc9x7G",
    "name": "get_weather",
    "arguments": "{\"location\":\"Tokyo, Japan\",\"unit\":\"c\"}"
  },
  {
    "type": "function_call_output",
    "call_id": "call_UbxMBIKVNhA8iVxWj7Qc9x7G",
    "output": "晴れ"
  },
  {
    "role": "assistant",
    "content": [
      {
        "type": "output_text",
        "text": "今日の東京の天気は晴れです。何か他に知りたいことがありますか？"
      }
    ]
  }
]
```

```json
{
  "model": "gpt-4o",
  "input": [
    {
      "role": "user",
      "content": [
        {
          "type": "input_text",
          "text": "現在の日本プロ野球の順位を調べてください。"
        }
      ]
    },
    {
      "type": "web_search_call",
      "id": "ws_67f22b7a6e8081929d6f15d6fef11b3703809705402c38a1",
      "status": "completed"
    },
    {
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "2025年4月6日現在、日本プロ野球（NPB）の2025年シーズンはまだ開幕していません。通常、NPBのシーズンは3月下旬から4月上旬に開幕しますが、今年の開幕日は未定です。そのため、現時点での順位情報はありません。\n\n参考までに、2024年シーズンの最終順位は以下の通りです。\n\n**セントラル・リーグ（セ・リーグ）**\n\n| 順位 | チーム名 | 試合数 | 勝利 | 敗戦 | 引分 | 勝率 | 勝差 |\n|---|---|---|---|---|---|---|---|\n| 1 | 巨人 | 143 | 77 | 59 | 7 | .566 | - |\n| 2 | 阪神 | 142 | 73 | 63 | 6 | .537 | 4.0 |\n| 3 | DeNA | 139 | 69 | 67 | 3 | .507 | 4.0 |\n| 4 | 広島 | 141 | 66 | 70 | 5 | .485 | 3.0 |\n| 5 | ヤクルト | 141 | 62 | 75 | 4 | .453 | 4.5 |\n| 6 | 中日 | 140 | 59 | 73 | 8 | .447 | 0.5 |\n\n**パシフィック・リーグ（パ・リーグ）**\n\n| 順位 | チーム名 | 試合数 | 勝利 | 敗戦 | 引分 | 勝率 | 勝差 |\n|---|---|---|---|---|---|---|---|\n| 1 | ソフトバンク | 141 | 89 | 49 | 3 | .645 | - |\n| 2 | 日本ハム | 140 | 73 | 59 | 8 | .553 | 13.0 |\n| 3 | ロッテ | 141 | 71 | 64 | 6 | .526 | 3.5 |\n| 4 | 楽天 | 138 | 66 | 69 | 3 | .489 | 5.0 |\n| 5 | オリックス | 142 | 62 | 77 | 3 | .446 | 6.0 |\n| 6 | 西武 | 142 | 49 | 91 | 2 | .350 | 13.5 |\n\n([baseball-jp.com](https://www.baseball-jp.com/rank/top.html?utm_source=openai))\n\n2025年シーズンの開幕日や最新の情報については、NPBの公式サイトや各球団の公式サイトをご確認ください。 "
        }
      ]
    }
  ],
  "text": {
    "format": {
      "type": "text"
    }
  },
  "reasoning": {},
  "tools": [
    {
      "type": "web_search_preview",
      "user_location": {
        "type": "approximate",
        "country": "JP"
      },
      "search_context_size": "low"
    }
  ],
  "temperature": 1,
  "max_output_tokens": 2048,
  "top_p": 1,
  "store": true
}
```

* リクエストルートオブジェクトの `input` フィールドが入力。
  * Text input: string | Input item list: array
  * 文字列を直接渡せるが、これはお手軽お試し用で、対話や function call ができない。
    以下 Input item list を考える。
* input の配列要素は以下のうちのどれか
  * Input message
  * Item
  * Item reference
* Item reference 他の Item を ID で参照するだけのようなので、考えなくてよい。
* Input message
  * type = `message`
    * Required ではないのでデフォルトが `message` っぽい。
      Playground の例では省略されている。
  * role
    * user | assistant | system | developer
    * system = developer で、おそらく互換用。
  * content
    * string | array
    * これまた文字列でも OK となっており、分かりにくい。
      テキストだけではなく、画像も入力できるようになったのが原因かもしれない。
      画像やファイル入力をしないのであれば string でいいのかもしれない。
    * array の場合の item は以下の混在可能な配列。
      * type = `input_text`, text: string
      * type = `input_image`, detail = high | low | auto, image_url, file_id
      * type = `input_file`, file_id, file_name, file_data
* Item:
  Input Message の配列としてしまうと input > output > input のような
  対話コンテキストや function call の経緯を指定できない。
  その場合にはこちらを使う。
  * Input message
    * type = `message`。同じじゃないか。
    * role, content も同じ。content の中身がややこしいのも同じ。
      role = user | system | developer。
    * status (optional) が増えていて紛らわしい。
      API から返ってきたものをそのまま使えるというだけで、特に意味は無さそう。
  * Output message
    * type = `message`。結局そんなに差はなさそう。
    * role = `assistant` only。構造体タイプではなくここの違いだけらしい。
    * content: 要素は以下
      * Output text
        * type = `output_text`
        * text: string
        * annotations: Required になっているが、例では指定されていない。。
          多分出力時のドキュメントをコピーしただけだと思う。
      * Refusal
        * type = `refusal`
        * これも以前の出力をそのまま渡せるということらしい。
          渡さなくていい気がする。
    * id, status: Input message に比べて増えており required と書かれているが、
      例では指定されていない。。API からの出力された時には必ずついてくるが、
      それを入力に渡すときは必須ではないということだろうか。
    * API document は非常に分かりにくいが、結局のところ Playground 例のように
      やればよさそう。
  * File search tool call
  * Computer tool call
  * Computer tool call output
  * Web search tool call
    * type = `web_search_call`
    * id, status しかない (例によって Required) が、Playground によると
      指定しているので id が必要なのかもしれない。
  * Function tool call
    * type = `function_call`
    * call_id
    * name
    * arguments
    * 他は optional でここはその通りっぽい。
  * Function tool call output
    * type = `function_call_output`
    * call_id
    * output
  * Reasoning
