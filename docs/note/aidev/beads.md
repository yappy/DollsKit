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

公式サイトの通り。
以下の他にも方法があるが、brew ならアップデートが楽かも。
ただ、データベースを扱うソフトウェアである以上、大きなバージョンアップ時には
マイグレーション作業が必要になるかも。

```sh
brew install beads           # macOS / Linux (recommended)
npm install -g @beads/bd     # Node.js users
```

## Shell Completion

`bd completion SH` でスクリプトが stdout に出力される。

```txt
Usage:
  bd completion [command]

Available Commands:
  bash        Generate the autocompletion script for bash
  fish        Generate the autocompletion script for fish
  powershell  Generate the autocompletion script for powershell
  zsh         Generate the autocompletion script for zsh
```

bash なら以下のコマンドを `.bashrc` とかに書けば OK。

```bash
source <(bd completion bash)
```

## 初期化

ローカル git リポジトリの中で `bd init`。
既にリモートリポジトリに beads データがある場合は `bd bootstrap`。

```sh
# Optional: refresh or install richer instructions for your agent
bd setup codex    # Codex CLI - installs skill, AGENTS.md guidance, and hooks
bd setup claude   # Claude Code - installs hooks/settings
bd setup factory  # Factory.ai Droid - creates/updates AGENTS.md
```

なんか各社 AI ごとの追加設定もある。
でも codex はやっても何も起きなかった気がする (init で最初から入る？)。

```txt
Contributing to someone else's repo? [y/N]:
```

`--role` オプションに "maintainer" or "contributor" の設定がある。
メンテナの場合はこの git リポジトリの remote をデータベース置き場として使う。
コントリビュータの場合はなんか別の場所に置く設定になる(未検証)。

```txt
  Auto-export can keep .beads/issues.jsonl up to date after write commands.
  This optional JSONL export is useful for viewers (bv), interchange, and issue-level migration.
  Dolt remotes/backups handle cross-machine sync and backup.

Enable auto-export? [y/N]:
```

これは以前マスターデータが Dolt ではなく jsonl ファイル (普通の git 管理)
だった頃の互換モード。
現在はマスターデータは通常の git branch とは別の場所で管理されており、
更新時に自動的に jsonl ファイルに書き出される。
`git commit` は自分でどうぞ。
旧方式のファイルを見ているビューワ等のサポートソフトウェアとの互換性のための機能。

成功するとなんか勝手に大量の AI への指示ファイルやフック設定などが追加され、
`git commit` までされる。`git show` で確認してみよう。
ローカルのデータベースは `.beads/` 以下に置かれ、`.gitignore` に追加される。

`git push` では beads データベースは push されない。
`git dolt push` / `git dolt pull` を使う。
git リポジトリは共有するが、通常の開発ブランチとは別れた場所に保管される。
(開発ブランチを切り替えてもデータベースは共有)

## 基本コマンド

公式 README のコピペ。

| Command | Action |
| --- | --- |
| `bd ready` | List tasks with no open blockers. |
| `bd create "Title" -p 0` | Create a P0 task. |
| `bd update <id> --claim` | Atomically claim a task (sets assignee + in_progress). |
| `bd dep add <child> <parent>` | Link tasks (blocks, related, parent-child). |
| `bd show <id>` | View task details and audit trail. |
| `bd prime` | Print agent workflow context and persistent memories. |
| `bd remember "insight"` | Store project memory that `bd prime` injects later. |

* 基本はイシュートラッカー。
* タスクを作成・状態変更することができる。
* タスク間には依存関係を張り、依存グラフとして管理することができる。
* `bd help CMD` or `bd CMD --help` でヘルプ表示。
  * ~~～ってどうやるの？みたいなのは AI に聞くと~~
    ~~AI がヘルプコマンドを実行して調べて教えてくれる。~~
  * ~~むしろ bd で～してって頼むと自分で調べてやってくれる。~~
* タイプ `--type`
  * bug
  * feature
  * task
  * epic
  * chore
  * decision
* 優先度 `--priority` は 0-4。0が最高優先度(critical)。
* ID はハッシュみたいなやつ。
* `update --claim` は担当者を自分に、ステータスを in_progress に変更する。
* 依存は blocks / blocked_by 以外にもいろいろある。
  * blocks
  * tracks
  * related
  * parent-child
  * discovered-from
  * until
  * caused-by
  * validates
  * relates-to
  * supersedes
* 1つのタスクの詳細を見たい場合は `show`
* `bd prime` は AI が初めに必ず読む内容。
* コンテキストを超えて永続的に保持したい事柄は、`bd remember` で追加できる。
  * `bd prime` の内容に追加されることで実現する。
  * `bd memories` で一覧表示。
  * `bd forget` で削除。
  * "日本語でお願いします" みたいな指示に有効。
  * ~~AI に頼めばやってくれる。~~

```txt
Working With Issues:
  assign            Assign an issue to someone
  children          List child beads of a parent
  close             Close one or more issues
  comment           Add a comment to an issue
  comments          View or manage comments on an issue
  create            Create a new issue (or batch from markdown/graph JSON)
  create-form       Create a new issue using an interactive form
  delete            Delete one or more issues and clean up references
  edit              Edit an issue field in $EDITOR
  gate              Manage async coordination gates
  label             Manage issue labels
  link              Link two issues with a dependency
  list              List issues
  merge-slot        Manage merge-slot gates for serialized conflict resolution
  note              Append a note to an issue
  priority          Set the priority of an issue
  promote           Promote a wisp to a permanent bead
  q                 Quick capture: create issue and output only ID
  query             Query issues using a simple query language
  reopen            Reopen one or more closed issues
  search            Search issues by text query
  set-state         Set operational state (creates event + updates label)
  show              Show issue details
  state             Query the current value of a state dimension
  tag               Add a label to an issue
  todo              Manage TODO items (convenience wrapper for task issues)
  update            Update one or more issues
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
