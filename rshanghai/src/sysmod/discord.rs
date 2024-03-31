//! Discord クライアント (bot) 機能。

use super::camera::{take_a_pic, TakePicOption};
use super::openai::{function::FunctionTable, Role};
use super::SystemModule;
use crate::sys::taskserver::WeakControl;
use crate::sys::{config, taskserver::Control};
use crate::sys::{taskserver, version};
use crate::sysmod::openai::function::FUNCTION_TOKEN;
use crate::sysmod::openai::{self, ChatMessage};
use crate::utils::chat_history::{self, ChatHistory};
use crate::utils::netutil::HttpStatusError;
use ::serenity::all::FullEvent;
use anyhow::{anyhow, ensure, Result};
use chrono::{NaiveTime, Utc};
use log::{error, info, warn};
use poise::samples::register_globally;
use poise::{serenity_prelude as serenity, FrameworkContext};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::builder::{CreateAttachment, CreateMessage};
use serenity::http::{Http, MessagePagination};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::Client;
use static_assertions::const_assert;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use time::Instant;

struct PoiseData {
    ctrl: WeakControl,
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
    /// パーミッションエラーメッセージ。
    /// オーナーのみ使用可能なコマンドを実行しようとした。
    perm_err_msg: String,
    /// パーミッションエラーを強制的に発生させる。デバッグ用。
    force_perm_err: bool,
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
            perm_err_msg: "バカジャネーノ".to_string(),
            force_perm_err: false,
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

fn convert_duration(mut min: u32) -> (u32, u32, u32) {
    let day = min / (60 * 24);
    min %= 60 * 24;
    let hour = min / 60;
    min %= 60;

    (day, hour, min)
}

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

impl Discord {
    /// コンストラクタ。
    ///
    /// 設定データの読み込みのみ行い、実際の初期化は async が有効になる
    /// [discord_main] で行う。
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[discord] initialize");

        let config = config::get(|cfg| cfg.discord.clone());
        let ai_config = config::get(|cfg| cfg.openai.clone());

        let mut auto_del_congig = BTreeMap::new();
        for &ch in &config.auto_del_chs {
            ensure!(ch != 0);
            auto_del_congig.insert(
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
            auto_del_config: auto_del_congig,
            chat_history,
            chat_timeout: None,
            func_table,
        })
    }

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

    // 自身の ID が on_mention 設定に必要なので別口で取得しておく
    /*
    let http = Http::new(&config.token);
    let info = http.get_current_application_info().await?;
    let myid = UserId::new(info.id.get());
    let ownerids = HashSet::from([info.owner.ok_or_else(|| anyhow!("No owner"))?.id]); */
    let ctrl_for_setup = ctrl.clone();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            on_error: |err| Box::pin(on_error(err)),
            event_handler: |ctx, ev, fctx, data| Box::pin(event_handler(ctx, ev, fctx, data)),
            prefix_options: poise::PrefixFrameworkOptions {
                //prefix: Some("~".into()),
                case_insensitive_commands: true,
                ..Default::default()
            },
            skip_checks_for_owners: true,
            // This is also where commands go
            commands: command_list(),
            ..Default::default()
        })
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
                    ctrl: Arc::downgrade(&ctrl_for_setup),
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
async fn reply_long(msg: &Message, ctx: &Context, content: &str) -> Result<()> {
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
            msg.reply(ctx, chunk).await?;
        }
        if fin {
            break;
        }
    }
    Ok(())
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
    vec![help(), sysinfo(), ai(), aistatus()]
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

/// AI assistant.
#[poise::command(slash_command, category = "AI")]
async fn ai(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    Ok(())
}

#[poise::command(slash_command, category = "AI", subcommands("aistatus_show", "aistatus_reset"))]
async fn aistatus(_ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    // 親コマンドはスラッシュコマンドでは使用不可
    Ok(())
}

#[poise::command(slash_command, category = "AI", rename = "show")]
async fn aistatus_show(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    ctx.reply("Not ready...").await?;

    Ok(())
}

#[poise::command(slash_command, category = "AI", rename = "reset")]
async fn aistatus_reset(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    ctx.reply("Not ready...").await?;

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

async fn on_error(error: poise::FrameworkError<'_, PoiseData, PoiseError>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
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

            let ctrl = if let Some(ctrl) = data.ctrl.upgrade() {
                ctrl
            } else {
                info!("[discord] already dropped");
                return Ok(());
            };
            let mut discord = ctrl.sysmods().discord.lock().await;
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
