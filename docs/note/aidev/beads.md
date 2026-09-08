# Beads

<https://github.com/gastownhall/beads>

AI 向け issue tracker。

人間向けの JIRA や GitHub Issues をプロトコル / API 経由で操作するより
AI フレンドリーで軽量、なんだと思う。
人間からも AI からも操作可能。依存グラフ機能付き。

バックエンドは Dolt という git によるバージョン管理付き SQL データベース。

<https://github.com/dolthub/dolt>

データベース本体は git リポジトリ内で管理される。
ソースコード管理ブランチとは別。

## インストール

```txt
  Welcome to Codex, OpenAI's command-line coding agent

  Sign in with ChatGPT to use Codex as part of your paid plan
  or connect an API key for usage-based billing

> 1. Sign in with ChatGPT
     Usage included with Plus, Pro, Business, and Enterprise plans

  2. Sign in with Device Code
     Sign in from another device with a one-time code

  3. Provide your own API key
     Pay for what you use
```

## Q & A (GPT-5.6 Luna)

> beads って昔はデータ本体が json っぽいものだったのが、
> のちに git sql 的なものに移行したという理解でよいですか？

はい、**概ねその理解で合っています**。ただし、Beads（`bd`）の歴史を少し正確に言うと、

> **JSONL + SQLite → Dolt（Git的なバージョン管理機能を持つSQL DB）**

という進化です。単純に「JSON → Git/SQL」ではありません。

### ざっくり時系列

初期の Beads は、

```text
.beads/
├── issues.jsonl     ← Gitで管理するデータ
└── beads.db         ← SQLite（ローカルで高速に検索するため）
```

という構成でした。

つまり、

```text
          Git
           │
     issues.jsonl
           │
     ┌─────┴─────┐
     ↓           ↓
  machine A   machine B
   SQLite      SQLite
```

という感じです。

`issues.jsonl` がGit上で同期され、各マシンではSQLiteに取り込んで使う、
という設計ですね。
実際、旧世代のBeadsについては「SQLiteが実行時DB、JSONLがGitで同期するデータ」
という説明がされています。

### その後、Doltへ

現在の公式 Beads は **Dolt** を使っています。

```text
.beads/
└── embeddeddolt/     ← 本体
```

Doltは面白いDBで、

> **SQL database + Gitっぽいバージョン管理**

というものです。

つまり、

```text
Git
 ├── commit
 ├── branch
 ├── diff
 └── merge

Dolt
 ├── commit
 ├── branch
 ├── diff
 ├── merge
 └── SQL
```

という発想です。
Beads自身も「Dolt is a version-controlled SQL database」と説明しています。

なので現在は、

```text
             Dolt
              │
       ┌──────┴──────┐
       │ SQL tables  │
       │ versioning  │
       │ branching   │
       │ merging     │
       └─────────────┘
```

が**データの本体**です。

一方、`issues.jsonl` は現在も残っていますが、これは本体ではなく、

> export / viewer / migration / interoperability 用

という位置づけになっています。

### なので「Git SQL的」という感覚はかなり正しい

むしろ現在のBeadsを一言で表現するなら、

「GitでJSONを管理するissue tracker」から「Gitのような履歴管理能力を持つ
SQL DBでissueを管理するtracker」へ進化した

と考えるとかなり分かりやすいです。

そしてこれには、**AIエージェントを複数動かす**というBeadsの用途がかなり関係しています。
DoltならSQLで

```sql
SELECT ...
FROM issues
WHERE status = 'open'
  AND priority < 2;
```

みたいな検索をしつつ、DBそのものにcommit/diff/mergeの概念を持たせられるので、SQLite + JSONL + Gitより自然に「複数agentが同じタスクDBをいじる」方向へ拡張できます。

ちなみに、**「昔のBeadsを使っていて、今のDolt版へどう移行するの？」**
というところは結構面白いです。
SQLite時代 → Dolt時代で実際にマイグレーションが存在します。
