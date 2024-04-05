//! Discord クライアント (bot) 機能。

use super::openai::{function::FunctionTable, Role};
use super::SystemModule;

use crate::sys::{config, taskserver::Control};
use crate::sys::{taskserver, version};
use crate::sysmod::camera::{self, TakePicOption};
use crate::sysmod::openai::function::FUNCTION_TOKEN;
use crate::sysmod::openai::{self, ChatMessage};
use crate::utils::chat_history::{self, ChatHistory};
use crate::utils::netutil::HttpStatusError;
use crate::utils::playtools::dice::{self};
use ::serenity::all::{CreateAttachment, FullEvent};
use anyhow::{anyhow, bail, ensure, Result};
use chrono::{NaiveTime, Utc};
use log::{error, info, warn};

use poise::{serenity_prelude as serenity, CreateReply, FrameworkContext};

use serde::{Deserialize, Serialize};

use serenity::http::MessagePagination;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::Client;

use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use time::Instant;

struct PoiseData {
    ctrl: Control,
}
type PoiseError = anyhow::Error;
type PoiseContext<'a> = poise::Context<'a, PoiseData, PoiseError>;

/// メッセージの最大文字数。 (Unicode codepoint)
const MSG_MAX_LEN: usize = 2000;

/// Discord 設定データ。toml 設定に対応する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// 機能を有効化するなら true。
    enabled: bool,
    /// アクセストークン。Developer Portal で入手できる。
    token: String,
    /// メッセージの発言先チャネル。
    /// Discord の詳細設定で開発者モードを有効にして、チャネルを右クリックで
    /// ID をコピーできる。
    notif_channel: u64,
    /// 自動削除機能の対象とするチャネル ID のリスト。
    auto_del_chs: Vec<u64>,
    /// オーナーのユーザ ID。
    /// Discord bot から得られるものは使わない。
    owner_ids: Vec<u64>,
    /// パーミッションエラーメッセージ。
    /// オーナーのみ使用可能なコマンドを実行しようとした。
    perm_err_msg: String,
    /// OpenAI プロンプト。
    #[serde(default)]
    prompt: DiscordPrompt,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: "".to_string(),
            notif_channel: 0,
            auto_del_chs: Default::default(),
            owner_ids: Default::default(),
            perm_err_msg: "バカジャネーノ".to_string(),
            prompt: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordPrompt {
    /// 最初に一度だけ与えられるシステムメッセージ。
    pub pre: Vec<String>,
    /// 個々のメッセージの直前に一度ずつ与えらえるシステムメッセージ。
    pub each: Vec<String>,
    /// 会話履歴をクリアするまでの時間。
    pub history_timeout_min: u32,
}

/// [DiscordPrompt] のデフォルト値。
const DEFAULT_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/res/openai_discord.toml"
));
impl Default for DiscordPrompt {
    fn default() -> Self {
        toml::from_str(DEFAULT_TOML).unwrap()
    }
}

/// Discord システムモジュール。
pub struct Discord {
    /// 設定データ。
    config: DiscordConfig,
    /// 定期実行の時刻リスト。
    wakeup_list: Vec<NaiveTime>,
    /// 現在有効な Discord Client コンテキスト。
    ///
    /// 起動直後は None で、[Handler::cache_ready] イベントの度に置き換わる。
    ctx: Option<Context>,
    /// [Self::ctx] が None の間に発言しようとしたメッセージのキュー。
    ///
    /// Some になるタイミングで全て送信する。
    postponed_msgs: Vec<String>,

    /// 自動削除機能の設定データ。
    auto_del_config: BTreeMap<ChannelId, AutoDeleteConfig>,

    /// ai コマンドの会話履歴。
    chat_history: ChatHistory,
    /// [Self::chat_history] の有効期限。
    chat_timeout: Option<Instant>,
    /// OpenAI function 機能テーブル
    func_table: FunctionTable<()>,
}

/// 自動削除設定。チャネルごとに保持される。
#[derive(Clone, Copy)]
pub struct AutoDeleteConfig {
    /// 残す数。0 は無効。
    keep_count: u32,
    /// 残す時間 (単位は分)。0 は無効。
    keep_dur_min: u32,
}

impl Display for AutoDeleteConfig {
    /// to_string 可能にする。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count_str = if self.keep_count != 0 {
            self.keep_count.to_string()
        } else {
            "Disabled".to_string()
        };
        let time_str = if self.keep_dur_min != 0 {
            let (d, h, m) = convert_duration(self.keep_dur_min);
            format!("{d} day(s) {h} hour(s) {m} minute(s)")
        } else {
            "Disabled".to_string()
        };

        write!(f, "Keep Count: {count_str}\nKeep Time: {time_str}")
    }
}

impl Discord {
    /// コンストラクタ。
    ///
    /// 設定データの読み込みのみ行い、実際の初期化は async が有効になる
    /// [discord_main] で行う。
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[discord] initialize");

        let config = config::get(|cfg| cfg.discord.clone());
        let ai_config = config::get(|cfg| cfg.openai.clone());

        let mut auto_del_config = BTreeMap::new();
        for &ch in &config.auto_del_chs {
            ensure!(ch != 0);
            auto_del_config.insert(
                ChannelId::new(ch),
                AutoDeleteConfig {
                    keep_count: 0,
                    keep_dur_min: 0,
                },
            );
        }

        // トークン上限を算出
        // Function 定義 + 前文 + (使用可能上限) + 出力
        let model_info = openai::get_model_info(&ai_config.model)?;
        let pre_token: usize = config
            .prompt
            .pre
            .iter()
            .map(|text| chat_history::token_count(text))
            .sum();
        let reserved = FUNCTION_TOKEN + pre_token + openai::get_output_reserved_token(model_info);
        assert!(reserved < model_info.token_limit);
        let chat_limit = model_info.token_limit - reserved;
        let chat_history = ChatHistory::new(chat_limit);
        info!("[discord] OpenAI token limit");
        info!("[discord] {:6} total", model_info.token_limit);
        info!("[discord] {reserved:6} reserved");
        info!("[discord] {:6} chat history", chat_limit);

        let mut func_table = FunctionTable::new();
        func_table.register_basic_functions();

        Ok(Self {
            config,
            wakeup_list,
            ctx: None,
            postponed_msgs: Default::default(),
            auto_del_config,
            chat_history,
            chat_timeout: None,
            func_table,
        })
    }

    /// 発言を投稿する。
    ///
    /// 接続前の場合、接続後まで遅延する。
    pub async fn say(&mut self, msg: &str) -> Result<()> {
        if !self.config.enabled {
            info!("[discord] disabled - msg: {}", msg);
            return Ok(());
        }
        if self.config.notif_channel == 0 {
            info!("[discord] notification disabled - msg: {}", msg);
            return Ok(());
        }
        if self.ctx.is_none() {
            info!("[discord] not ready, postponed - msg: {}", msg);
            self.postponed_msgs.push(msg.to_string());
            return Ok(());
        }

        info!("[discord] say msg: {}", msg);
        let ch = ChannelId::new(self.config.notif_channel);
        let ctx = self.ctx.as_ref().unwrap();
        ch.say(ctx, msg).await?;

        Ok(())
    }

    /// [Self::chat_history] にタイムアウトを適用する。
    fn check_history_timeout(&mut self) {
        let now = Instant::now();

        if let Some(timeout) = self.chat_timeout {
            if now > timeout {
                self.chat_history.clear();
                self.chat_timeout = None;
            }
        }
    }
}

/// システムを初期化し開始する。
///
/// [Discord::on_start] から spawn される。
async fn discord_main(ctrl: Control) -> Result<()> {
    let (config, wakeup_list) = {
        let discord = ctrl.sysmods().discord.lock().await;

        (discord.config.clone(), discord.wakeup_list.clone())
    };

    // owner_ids を HashSet に変換 (0 は panic するので禁止)
    let mut owners = HashSet::new();
    for id in config.owner_ids {
        if id == 0 {
            bail!("owner id must not be 0");
        }
        owners.insert(UserId::new(id));
    }
    info!("[discord] owners: {:?}", owners);

    let ctrl_for_setup = ctrl.clone();
    let framework = poise::Framework::builder()
        // owner は手動で設定する
        .initialize_owners(false)
        // その他オプション
        .options(poise::FrameworkOptions {
            on_error: |err| Box::pin(on_error(err)),
            pre_command: |ctx| Box::pin(pre_command(ctx)),
            post_command: |ctx| Box::pin(post_command(ctx)),
            event_handler: |ctx, ev, fctx, data| Box::pin(event_handler(ctx, ev, fctx, data)),
            // prefix コマンドは使っていない
            prefix_options: poise::PrefixFrameworkOptions {
                //prefix: Some("~".into()),
                //case_insensitive_commands: true,
                ..Default::default()
            },
            // owner は手動で設定する (builder の方から設定されるようだがデフォルトが true なので念のためこちらも)
            initialize_owners: false,
            owners,
            // コマンドリスト
            commands: command_list(),
            ..Default::default()
        })
        // ハンドラ
        .setup(|ctx, _ready, framework| {
            // 最初の ready イベントで呼ばれる
            Box::pin(async move {
                let mut discord = ctrl_for_setup.sysmods().discord.lock().await;
                discord.ctx = Some(ctx.clone());

                info!("[discord] register commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("[discord] register commands OK");

                // construct user data here (invoked when bot connects to Discord)
                Ok(PoiseData {
                    ctrl: Arc::clone(&ctrl_for_setup),
                })
            })
        })
        .build();

    // クライアントの初期化
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(config.token.clone(), intents)
        .framework(framework)
        .await?;

    // システムシャットダウンに対応してクライアントにシャットダウン要求を送る
    // 別タスクを立ち上げる
    let ctrl_for_cancel = Arc::clone(&ctrl);
    let shard_manager = client.shard_manager.clone();
    taskserver::spawn_oneshot_fn(&ctrl, "discord-exit", async move {
        ctrl_for_cancel.wait_cancel_rx().await;
        info!("[discord-exit] recv cancel");
        shard_manager.shutdown_all().await;
        info!("[discord-exit] shutdown_all ok");
        // shutdown_all が完了した後は ready は呼ばれないはずなので
        // ここで ctx を処分する
        // ctx.data に Control を持たせているので ctx がリークしたままだと
        // 終了処理が完了しない
        let ctx = ctrl_for_cancel.sysmods().discord.lock().await.ctx.take();
        drop(ctx);
        info!("[discord-exit] context dropped");

        Ok(())
    });

    // 定期チェックタスクを立ち上げる
    taskserver::spawn_periodic_task(&ctrl, "discord-periodic", &wakeup_list, periodic_main);

    // システムスタート
    client.start().await?;
    info!("[discord] client exit");

    Ok(())
}

/// 文字数制限に気を付けつつ分割して送信する。
async fn reply_long(ctx: &PoiseContext<'_>, content: &str) -> Result<()> {
    // mention 関連でのずれが少し怖いので余裕を持たせる
    const LEN: usize = MSG_MAX_LEN - 128;

    let mut remain = content;
    loop {
        let (chunk, fin) = match remain.char_indices().nth(LEN) {
            Some((ind, _c)) => {
                let (a, b) = remain.split_at(ind);
                remain = b;

                (a, false)
            }
            None => (remain, true),
        };
        if !chunk.is_empty() {
            ctx.reply(chunk).await?;
        }
        if fin {
            break;
        }
    }
    Ok(())
}

/// Markdown エスケープしながら Markdown 引用する。
/// 文字数制限に気を付けつつ分割して送信する。
async fn reply_long_mdquote(ctx: &PoiseContext<'_>, content: &str) -> Result<()> {
    // mention 関連でのずれが少し怖いので余裕を持たせる
    // 引用符の分も含める
    const LEN: usize = MSG_MAX_LEN - 128;
    const QUOTE_PRE: &str = "```\n";
    const QUOTE_PST: &str = "\n```";
    const SPECIALS: &str = "#+-*_\\`.!{}[]()";

    let mut count = 0;
    let mut buf = String::from(QUOTE_PRE);
    for c in content.chars() {
        if count >= LEN {
            buf.push_str(QUOTE_PST);
            ctx.reply(buf).await?;

            count = 0;
            buf = String::from(QUOTE_PRE);
        }
        if SPECIALS.find(c).is_some() {
            buf.push('\\');
        }
        buf.push(c);
        count += 1;
    }
    if !buf.is_empty() {
        buf.push_str(QUOTE_PST);
        ctx.reply(buf).await?;
    }
    Ok(())
}

/// 分を (日, 時間, 分) に変換する。
fn convert_duration(mut min: u32) -> (u32, u32, u32) {
    let day = min / (60 * 24);
    min %= 60 * 24;
    let hour = min / 60;
    min %= 60;

    (day, hour, min)
}

/// 日時分からなる文字列を分に変換する。
///
/// 例: 1d2h3m
fn parse_duration(src: &str) -> Result<u32> {
    let mut min = 0u32;
    let mut buf = String::new();
    for c in src.chars() {
        if c == 'd' || c == 'h' || c == 'm' {
            let n: u32 = buf.parse()?;
            let n = match c {
                'd' => n.saturating_mul(24 * 60),
                'h' => n.saturating_mul(60),
                'm' => n,
                _ => panic!(),
            };
            min = min.saturating_add(n);
            buf.clear();
        } else {
            buf.push(c);
        }
    }
    Ok(min)
}

/// チャネル内の全メッセージを取得し、フィルタ関数が true を返したものを
/// すべて削除する。
///
/// Bulk delete 機能で一気に複数を消せるが、2週間以上前のメッセージが
/// 含まれていると BAD REQUEST になる等扱いが難しいので rate limit は
/// 気になるが1つずつ消す。
///
/// * `ctx` - HTTP コンテキスト。
/// * `ch` - Channel ID。
/// * `filter` - (メッセージ, 番号, 総数) から消すならば true を返す関数。
///
/// (消した数, 総メッセージ数) を返す。
async fn delete_msgs_in_channel<F: Fn(&Message, usize, usize) -> bool>(
    ctx: &Context,
    ch: ChannelId,
    filter: F,
) -> Result<(usize, usize)> {
    // id=0 から 100 件ずつすべてのメッセージを取得する
    let mut allmsgs = BTreeMap::<MessageId, Message>::new();
    const GET_MSG_LIMIT: u8 = 100;
    let mut after = None;
    loop {
        // https://discord.com/developers/docs/resources/channel#get-channel-messages
        info!("get_messages: after={:?}", after);
        let target = after.map(MessagePagination::After);
        let msgs = ctx
            .http
            .get_messages(ch, target, Some(GET_MSG_LIMIT))
            .await?;
        // 空配列ならば完了
        if msgs.is_empty() {
            break;
        }
        // 降順で送られてくるので ID でソートし直す
        allmsgs.extend(msgs.iter().map(|m| (m.id, m.clone())));
        // 最後の message id を次回の after に設定する
        after = Some(*allmsgs.keys().next_back().unwrap());
    }
    info!("obtained {} messages", allmsgs.len());

    let mut delcount = 0_usize;
    for (i, (&mid, msg)) in allmsgs.iter().enumerate() {
        if !filter(msg, i, allmsgs.len()) {
            continue;
        }
        // ch, msg ID はログに残す
        info!("Delete: ch={}, msg={}", ch, mid);
        // https://discord.com/developers/docs/resources/channel#delete-message
        ctx.http.delete_message(ch, mid, None).await?;
        delcount += 1;
    }
    info!("deleted {} messages", delcount);

    Ok((delcount, allmsgs.len()))
}

/// 自動削除周期タスク。
async fn periodic_main(ctrl: Control) -> Result<()> {
    let (ctx, config_map) = {
        let discord = ctrl.sysmods().discord.lock().await;
        if discord.ctx.is_none() {
            // ready 前なら何もせず正常終了する
            return Ok(());
        }
        (
            discord.ctx.as_ref().unwrap().clone(),
            discord.auto_del_config.clone(),
        )
    };

    // UNIX timestamp [sec]
    let now = Utc::now().timestamp() as u64;

    for (ch, config) in config_map {
        let AutoDeleteConfig {
            keep_count,
            keep_dur_min,
        } = config;
        if keep_count == 0 && keep_dur_min == 0 {
            continue;
        }
        let keep_dur_sec = (keep_dur_min as u64).saturating_mul(60);
        let (_delcount, _total) = delete_msgs_in_channel(&ctx, ch, |msg, i, len| {
            let mut keep = true;
            if keep_count != 0 {
                keep = keep && i + (keep_count as usize) < len;
            }
            if keep_dur_min != 0 {
                let created = msg.timestamp.timestamp() as u64;
                // u64 [sec] 同士の減算で経過時間を計算する
                // オーバーフローは代わりに 0 とする
                let duration = now.saturating_sub(created);
                keep = keep && duration <= keep_dur_sec;
            }
            !keep
        })
        .await?;
    }

    Ok(())
}

//------------------------------------------------------------------------------
// command
// https://docs.rs/poise/latest/poise/macros/attr.command.html
//------------------------------------------------------------------------------

fn command_list() -> Vec<poise::Command<PoiseData, PoiseError>> {
    vec![
        help(),
        sysinfo(),
        autodel(),
        coin(),
        dice(),
        attack(),
        camera(),
        ai(),
        aistatus(),
        aiimg(),
    ]
}

/// Show command help.
#[poise::command(slash_command, category = "General")]
pub async fn help(
    ctx: PoiseContext<'_>,
    #[description = "Command name"] command: Option<String>,
) -> Result<(), PoiseError> {
    let config = poise::builtins::HelpConfiguration {
        // その人だけに見える返信にするかどうか
        ephemeral: false,
        show_subcommands: true,
        extra_text_at_bottom: "",
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}

/// Show system information.
#[poise::command(slash_command, category = "General")]
async fn sysinfo(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let ver_info: &str = version::version_info();
    ctx.reply(ver_info).await?;

    Ok(())
}

const AUTODEL_INVALID_CH_MSG: &str = "Auto delete feature is not enabled for this channel.
Please contact my owner.";

#[poise::command(
    slash_command,
    category = "Auto Delete",
    subcommands("autodel_status", "autodel_set")
)]
async fn autodel(_ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    // 親コマンドはスラッシュコマンドでは使用不可
    Ok(())
}

/// Get the auto-delete status in this channel.
#[poise::command(slash_command, category = "Auto Delete", rename = "status")]
async fn autodel_status(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let ch = ctx.channel_id();
    let config = {
        let data = ctx.data();
        let discord = data.ctrl.sysmods().discord.lock().await;

        discord.auto_del_config.get(&ch).copied()
    };

    if let Some(config) = config {
        ctx.reply(format!("{config}")).await?;
    } else {
        ctx.reply(AUTODEL_INVALID_CH_MSG).await?;
    }

    Ok(())
}

/// Enable/Disable/Config auto-delete feature in this channel.
///
/// "0 0" disables the feature.
#[poise::command(slash_command, category = "Auto Delete", rename = "set")]
async fn autodel_set(
    ctx: PoiseContext<'_>,
    #[description = "Delete old messages other than this count of newer ones (0: disable)"]
    keep_count: u32,
    #[description = "Delete messages after this time (e.g. 1d, 3h, 30m, 1d23h59m, etc.) (0: disable)"]
    keep_duration: String,
) -> Result<(), PoiseError> {
    let ch = ctx.channel_id();
    let keep_duration = parse_duration(&keep_duration);
    if keep_duration.is_err() {
        ctx.reply("keep_duration parse error.").await?;
        return Ok(());
    }
    let keep_duration = keep_duration.unwrap();

    let msg = {
        let mut discord = ctx.data().ctrl.sysmods().discord.lock().await;

        let config = discord.auto_del_config.get_mut(&ch);
        match config {
            Some(config) => {
                config.keep_count = keep_count;
                config.keep_dur_min = keep_duration;
                format!("OK\n{config}")
            }
            None => AUTODEL_INVALID_CH_MSG.to_string(),
        }
    };
    ctx.reply(msg).await?;

    Ok(())
}

/// Flip coin(s).
#[poise::command(slash_command, category = "Play Tools")]
async fn coin(
    ctx: PoiseContext<'_>,
    #[description = "Dice count (default=1)"] count: Option<u32>,
) -> Result<(), PoiseError> {
    let count = count.unwrap_or(1);

    let msg = match dice::roll(2, count) {
        Ok(v) => {
            let mut buf = format!("Flip {count} coin(s)\n");
            buf.push('[');
            let mut first = true;
            for n in v {
                if first {
                    first = false;
                } else {
                    buf.push(',');
                }
                buf.push_str(if n == 1 { "\"H\"" } else { "\"T\"" });
            }
            buf.push(']');
            buf
        }
        Err(err) => err.to_string(),
    };
    ctx.reply(msg).await?;

    Ok(())
}

/// Roll dice.
#[poise::command(slash_command, category = "Play Tools")]
async fn dice(
    ctx: PoiseContext<'_>,
    #[description = "Face count (default=6)"] face: Option<u64>,
    #[description = "Dice count (default=1)"] count: Option<u32>,
) -> Result<(), PoiseError> {
    let face = face.unwrap_or(6);
    let count = count.unwrap_or(1);

    let msg = match dice::roll(face, count) {
        Ok(v) => {
            let mut buf = format!("Roll {count} dice with {face} face(s)\n");
            buf.push('[');
            let mut first = true;
            for n in v {
                if first {
                    first = false;
                } else {
                    buf.push(',');
                }
                buf.push_str(&n.to_string());
            }
            buf.push(']');
            buf
        }
        Err(err) => err.to_string(),
    };
    ctx.reply(msg).await?;

    Ok(())
}

/// Order the assistant to say something.
///
/// You can specify target user(s).
#[poise::command(slash_command, category = "Manipulation", owners_only)]
async fn attack(
    ctx: PoiseContext<'_>,
    #[description = "Target user"] target: Option<UserId>,
    #[description = "Chat message to be said"]
    #[min_length = 1]
    #[max_length = 1024]
    chat_msg: String,
) -> Result<(), PoiseError> {
    let text = if let Some(user) = target {
        format!("{} {}", user.mention(), chat_msg)
    } else {
        chat_msg
    };

    info!("[discord] reply: {text}");
    ctx.reply(text).await?;
    Ok(())
}

/// Take a picture.
#[poise::command(slash_command, category = "Manipulation", owners_only)]
async fn camera(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    ctx.reply("Taking a picture...").await?;

    let pic = camera::take_a_pic(TakePicOption::new()).await?;

    let attach = CreateAttachment::bytes(&pic[..], "camera.jpg");
    ctx.send(
        CreateReply::default()
            .content("camera.jpg")
            .attachment(attach),
    )
    .await?;

    Ok(())
}

/// AI assistant.
///
/// The owner of the assistant will pay the usage fee for ChatGPT.
#[poise::command(slash_command, category = "AI")]
async fn ai(
    ctx: PoiseContext<'_>,
    #[description = "Chat message to AI assistant"]
    #[min_length = 1]
    #[max_length = 1024]
    chat_msg: String,
    #[description = "Show internal details when AI calls a function. (default=False)"]
    trace_function_call: Option<bool>,
) -> Result<(), PoiseError> {
    let data = ctx.data();
    let mut discord = data.ctrl.sysmods().discord.lock().await;

    // タイムアウト処理
    discord.check_history_timeout();

    // そのまま引用返信
    reply_long_mdquote(&ctx, &chat_msg).await?;

    // 今回の発言をヒストリに追加 (システムメッセージ + 本体)
    let sysmsg = discord
        .config
        .prompt
        .each
        .join("")
        .replace("${user}", &ctx.author().name);
    discord.chat_history.push({
        ChatMessage {
            role: Role::System,
            content: Some(sysmsg),
            ..Default::default()
        }
    });
    discord.chat_history.push(ChatMessage {
        role: Role::User,
        content: Some(chat_msg.to_string()),
        ..Default::default()
    });

    let reply_msg = loop {
        let ai = data.ctrl.sysmods().openai.lock().await;

        // 送信用リスト
        let mut all_msgs = Vec::new();
        // 先頭システムメッセージ
        all_msgs.push(ChatMessage {
            role: Role::System,
            content: Some(discord.config.prompt.pre.join("")),
            ..Default::default()
        });
        // それ以降 (ヒストリの中身全部) を追加
        for m in discord.chat_history.iter() {
            all_msgs.push(m.clone());
        }
        // ChatGPT API
        let reply_msg = ai
            .chat_with_function(&all_msgs, discord.func_table.function_list())
            .await;
        match &reply_msg {
            Ok(reply) => {
                // 応答を履歴に追加
                discord.chat_history.push(reply.clone());
                if reply.function_call.is_some() {
                    // function call が返ってきた
                    let func_name = &reply.function_call.as_ref().unwrap().name;
                    let func_args = &reply.function_call.as_ref().unwrap().arguments;
                    let func_res = discord.func_table.call((), func_name, func_args).await;
                    // trace
                    if trace_function_call.unwrap_or(false) {
                        reply_long(
                            &ctx,
                            &format!(
                                "function call: {func_name}\nparameters: {func_args}\nresult: {}",
                                func_res.content.as_ref().unwrap()
                            ),
                        )
                        .await?;
                    }
                    // function 応答を履歴に追加
                    discord.chat_history.push(func_res);
                    // continue
                } else {
                    // 通常の応答が返ってきた
                    break reply_msg;
                }
            }
            Err(err) => {
                // エラーが発生した
                error!("{:#?}", err);
                break reply_msg;
            }
        }
    };

    // discord 返信
    match reply_msg {
        Ok(reply_msg) => {
            let text = reply_msg
                .content
                .ok_or_else(|| anyhow!("content required"))?;
            info!("[discord] openai reply: {text}");
            reply_long(&ctx, &text).await?;

            // タイムアウト延長
            discord.chat_timeout = Some(
                Instant::now()
                    + Duration::from_secs(discord.config.prompt.history_timeout_min as u64 * 60),
            );
        }
        Err(err) => {
            error!("[discord] openai error: {:#?}", err);
            // HTTP status が得られるタイプのエラーのみ discord 返信する
            if let Some(err) = err.downcast_ref::<HttpStatusError>() {
                warn!("[discord] openai reply: {} {}", err.status, err.body);
                let reply_msg = format!("OpenAI API Error, HTTP Status: {}", err.status);
                ctx.reply(reply_msg.to_string()).await?;
            }
        }
    }

    Ok(())
}

#[poise::command(
    slash_command,
    category = "AI",
    subcommands("aistatus_show", "aistatus_reset", "aistatus_funclist")
)]
async fn aistatus(_ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    // 親コマンドはスラッシュコマンドでは使用不可
    Ok(())
}

/// Show AI chat history status.
#[poise::command(slash_command, category = "AI", rename = "show")]
async fn aistatus_show(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let text = {
        let ctrl = &ctx.data().ctrl;
        let mut discord = ctrl.sysmods().discord.lock().await;

        discord.check_history_timeout();
        format!(
            "History: {}\nToken: {} / {}, Timeout: {} min",
            discord.chat_history.len(),
            discord.chat_history.usage().0,
            discord.chat_history.usage().1,
            discord.config.prompt.history_timeout_min
        )
    };
    ctx.reply(text).await?;

    Ok(())
}

/// Clear AI chat history status.
#[poise::command(slash_command, category = "AI", rename = "reset")]
async fn aistatus_reset(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    {
        let ctrl = &ctx.data().ctrl;
        let mut discord = ctrl.sysmods().discord.lock().await;

        discord.chat_history.clear();
    }
    ctx.reply("OK").await?;

    Ok(())
}

/// Show AI function list.
/// You can request the assistant to call these functions.
#[poise::command(slash_command, category = "AI", rename = "funclist")]
async fn aistatus_funclist(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let help = {
        let discord = ctx.data().ctrl.sysmods().discord.lock().await;

        discord.func_table.create_help()
    };
    let text = format!("```\n{help}\n```");
    ctx.reply(text).await?;

    Ok(())
}

/// AI image generation.
#[poise::command(slash_command, category = "AI")]
async fn aiimg(
    ctx: PoiseContext<'_>,
    #[description = "Prompt string"]
    #[min_length = 1]
    #[max_length = 1024]
    prompt: String,
) -> Result<(), PoiseError> {
    // そのまま引用返信
    reply_long_mdquote(&ctx, &prompt).await?;

    let img_url = {
        let ctrl = &ctx.data().ctrl;
        let ai = ctrl.sysmods().openai.lock().await;

        let mut resp = ai.generate_image(&prompt, 1).await?;
        resp.pop().ok_or_else(|| anyhow!("image array too short"))?
    };

    ctx.reply(img_url).await?;

    Ok(())
}

impl SystemModule for Discord {
    /// async 使用可能になってからの初期化。
    ///
    /// 設定有効ならば [discord_main] を spawn する。
    fn on_start(&self, ctrl: &Control) {
        info!("[discord] on_start");
        if self.config.enabled {
            taskserver::spawn_oneshot_task(ctrl, "discord", discord_main);
        }
    }
}

/// Poise イベントハンドラ。
async fn pre_command(ctx: PoiseContext<'_>) {
    info!(
        "[discord] command {} from {:?} {:?}",
        ctx.command().name,
        ctx.author().name,
        ctx.author().global_name.as_deref().unwrap_or("?")
    );
    info!("[discord] {:?}", ctx.invocation_string());
}

async fn post_command(ctx: PoiseContext<'_>) {
    info!(
        "[discord] command {} from {:?} {:?} OK",
        ctx.command().name,
        ctx.author().name,
        ctx.author().global_name.as_deref().unwrap_or("?")
    );
}

/// Poise イベントハンドラ。
///
/// [poise::builtins::on_error] のままでまずい部分を自分でやる。
async fn on_error(error: poise::FrameworkError<'_, PoiseData, PoiseError>) {
    // エラーを返していないはずのものは panic にする
    match error {
        poise::FrameworkError::Setup { error, .. } => {
            panic!("Failed on setup: {:#?}", error)
        }
        poise::FrameworkError::EventHandler { error, .. } => {
            panic!("Failed on eventhandler: {:#?}", error)
        }
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!(
                "[discord] error in command `{}`: {:#?}",
                ctx.command().name,
                error
            );
        }
        poise::FrameworkError::NotAnOwner { ctx, .. } => {
            let errmsg = ctx
                .data()
                .ctrl
                .sysmods()
                .discord
                .lock()
                .await
                .config
                .perm_err_msg
                .clone();
            info!("[discord] not an owner: {}", ctx.author());
            info!("[discord] reply: {errmsg}");
            if let Err(why) = ctx.reply(errmsg).await {
                error!("[discord] reply error: {:#?}", why)
            }
        }
        poise::FrameworkError::UnknownInteraction { interaction, .. } => {
            warn!(
                "[discord] received unknown interaction \"{}\"",
                interaction.data.name
            );
        }
        error => {
            if let Err(why) = poise::builtins::on_error(error).await {
                error!("[discord] error while handling error: {:#?}", why)
            }
        }
    }
}

/// Serenity の全イベントハンドラ。
///
/// Poise のコンテキストが渡されるので、Serenity ではなく Poise の
/// [FrameworkOptions] 経由で設定する。
async fn event_handler(
    ctx: &Context,
    ev: &FullEvent,
    _fctx: FrameworkContext<'_, PoiseData, PoiseError>,
    data: &PoiseData,
) -> Result<(), PoiseError> {
    match ev {
        FullEvent::Ready { data_about_bot } => {
            info!("[discord] connected as {}", data_about_bot.user.name);
            Ok(())
        }
        FullEvent::Resume { event: _ } => {
            info!("[discord] resumed");
            Ok(())
        }
        FullEvent::CacheReady { guilds } => {
            // このタイミングで [Discord::ctx] に ctx をクローンして保存する。
            info!("[discord] cache ready - guild: {}", guilds.len());

            let mut discord = data.ctrl.sysmods().discord.lock().await;
            let ctx_clone = ctx.clone();
            discord.ctx = Some(ctx_clone);

            info!(
                "[discord] send postponed msgs ({})",
                discord.postponed_msgs.len()
            );
            for msg in &discord.postponed_msgs {
                let ch = discord.config.notif_channel;
                // notif_channel が有効の場合しかキューされない
                assert_ne!(0, ch);

                info!("[discord] say msg: {}", msg);
                let ch = ChannelId::new(ch);
                if let Err(why) = ch.say(&ctx, msg).await {
                    error!("{:#?}", why);
                }
            }
            discord.postponed_msgs.clear();
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_toml() {
        // should not panic
        let obj: DiscordPrompt = Default::default();
        assert_ne!(obj.pre.len(), 0);
        assert_ne!(obj.each.len(), 0);
    }

    #[test]
    fn convert_auto_del_time() {
        let (d, h, m) = convert_duration(0);
        assert_eq!(d, 0);
        assert_eq!(h, 0);
        assert_eq!(m, 0);

        let (d, h, m) = convert_duration(3 * 24 * 60 + 23 * 60 + 59);
        assert_eq!(d, 3);
        assert_eq!(h, 23);
        assert_eq!(m, 59);
    }
}
