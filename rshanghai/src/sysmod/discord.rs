//! Discord クライアント (bot) 機能。

use super::camera::{take_a_pic, TakePicOption};
use super::openai::{function::FunctionTable, Role};
use super::SystemModule;
use crate::sys::version;
use crate::sys::{config, taskserver::Control};
use crate::sysmod::openai::function::FUNCTION_TOKEN;
use crate::sysmod::openai::{self, ChatMessage};
use crate::utils::chat_history::{self, ChatHistory};
use crate::utils::netutil::HttpStatusError;
use anyhow::{anyhow, ensure, Result};
use chrono::{NaiveTime, Utc};
use log::{error, info, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::builder::{CreateAttachment, CreateMessage};
use serenity::framework::standard::macros::{command, group, help, hook};
use serenity::framework::standard::{
    help_commands, Args, CommandError, CommandGroup, CommandResult, Configuration, HelpOptions,
};
use serenity::http::{Http, MessagePagination};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{framework::StandardFramework, Client};
use static_assertions::const_assert;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::time::Duration;
use time::Instant;

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
    let http = Http::new(&config.token);
    let info = http.get_current_application_info().await?;
    let myid = UserId::new(info.id.get());
    let ownerids = HashSet::from([info.owner.ok_or_else(|| anyhow!("No owner"))?.id]);

    let framework = StandardFramework::new()
        // コマンド前後でのフック (ロギング用)
        .before(before_hook)
        .after(after_hook)
        .unrecognised_command(unrecognised_hook)
        // コマンドとヘルプの登録
        .group(&GENERAL_GROUP)
        .help(&MY_HELP);
    // コマンドのプレフィクスはなし
    // bot へのメンションをトリガとする
    framework.configure(
        Configuration::new()
            .prefix("")
            .on_mention(Some(myid))
            .owners(ownerids.clone()),
    );

    // クライアントの初期化
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(config.token.clone(), intents)
        .event_handler(Handler)
        .framework(framework)
        .await?;

    // グローバルデータの設定
    {
        let mut data = client.data.write().await;

        data.insert::<ControlData>(ctrl.clone());
        data.insert::<ConfigData>(config);
        data.insert::<OwnerData>(ownerids);
    }

    // システムシャットダウンに対応してクライアントにシャットダウン要求を送る
    // 別タスクを立ち上げる
    let mut ctrl_for_cancel = ctrl.clone();
    let shard_manager = client.shard_manager.clone();
    ctrl.spawn_oneshot_fn("discord-exit", async move {
        ctrl_for_cancel.cancel_rx().changed().await.unwrap();
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
    ctrl.spawn_periodic_task("discord-periodic", &wakeup_list, periodic_main);

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

/// コマンド開始前のフック。ロギング用。
#[hook]
async fn before_hook(_: &Context, msg: &Message, cmd_name: &str) -> bool {
    info!("[discord] command {} by {}", cmd_name, msg.author.name);

    true
}

/// コマンド完了後のフック。ロギング用。
#[hook]
async fn after_hook(_: &Context, _: &Message, cmd_name: &str, result: Result<(), CommandError>) {
    match result {
        Ok(()) => {
            info!("[discord] command {} succeeded", cmd_name);
        }
        Err(why) => {
            error!("[discord] error in {}: {:?}", cmd_name, why);
        }
    };
}

/// コマンド認識不能時のフック。ロギング用。
#[hook]
async fn unrecognised_hook(_: &Context, msg: &Message, cmd_name: &str) {
    warn!(
        "[discord] unknown command {} by {}",
        cmd_name, msg.author.name
    );
}

impl SystemModule for Discord {
    /// async 使用可能になってからの初期化。
    ///
    /// 設定有効ならば [discord_main] を spawn する。
    fn on_start(&self, ctrl: &Control) {
        info!("[discord] on_start");
        if self.config.enabled {
            ctrl.spawn_oneshot_task("discord", discord_main);
        }
    }
}

struct ControlData;
impl TypeMapKey for ControlData {
    type Value = Control;
}

struct ConfigData;
impl TypeMapKey for ConfigData {
    type Value = DiscordConfig;
}

struct OwnerData;
impl TypeMapKey for OwnerData {
    type Value = HashSet<UserId>;
}

struct Handler;

/// Discord クライアントとしてのイベントハンドラ。
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("[discord] connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("[discord] resumed");
    }

    /// このタイミングで [Discord::ctx] に ctx をクローンして保存する。
    /// [Discord::postponed_msgs] があれば全て送信する。
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        info!("[discord] cache ready - guild: {}", guilds.len());

        let ctx_clone = ctx.clone();
        let data = ctx.data.read().await;
        let ctrl = data.get::<ControlData>().unwrap();
        let mut discord = ctrl.sysmods().discord.lock().await;
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
    }
}

#[help]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await?;

    Ok(())
}

async fn owner_check(ctx: &Context, msg: &Message) -> CommandResult {
    let (accept, errmsg) = {
        let data = ctx.data.read().await;
        let owners = data.get::<OwnerData>().unwrap();
        let config = data.get::<ConfigData>().unwrap();
        let accept = !config.force_perm_err && owners.contains(&msg.author.id);
        let errmsg = config.perm_err_msg.clone();

        (accept, errmsg)
    };

    if accept {
        Ok(())
    } else {
        if let Err(why) = msg.reply(ctx, errmsg).await {
            warn!("error on reply: {:#}", why);
        }

        Err(anyhow!("permission error").into())
    }
}

#[group]
#[sub_groups(autodel)]
#[commands(sysinfo, dice, delmsg, camera, attack, ai, aistatus, aireset, aiimg)]
struct General;

#[command]
#[description("Print system information.")]
#[usage("")]
#[example("")]
async fn sysinfo(ctx: &Context, msg: &Message) -> CommandResult {
    let ver_info: &str = version::version_info();
    msg.reply(ctx, ver_info).await?;

    Ok(())
}

/// ダイスの面数の最大値。
const DICE_MAX: u64 = 1u64 << 56;
/// ダイスの個数の最大値。
const DICE_COUNT_MAX: u64 = 100u64;
const_assert!(DICE_MAX < u64::MAX / DICE_COUNT_MAX);

/// ダイスロール機能のコア。
///
/// * `dice` - 何面のダイスを振るか。
/// * `count` - 何個のダイスを振るか。
fn dice_core(dice: u64, count: u64) -> Vec<u64> {
    assert!((1..=DICE_MAX).contains(&dice));
    assert!((1..=DICE_COUNT_MAX).contains(&count));

    let mut result = vec![];
    let mut rng = rand::thread_rng();
    for _ in 0..count {
        result.push(rng.gen_range(1..=dice));
    }

    result
}

#[command]
#[description("Roll a dice with 1-**dice** faces **count** times.")]
#[description("Default: dice=6, count=1")]
#[usage("[dice] [count]")]
#[example("")]
#[example("6 2")]
#[min_args(0)]
#[max_args(2)]
async fn dice(ctx: &Context, msg: &Message, mut arg: Args) -> CommandResult {
    let d = if !arg.is_empty() { arg.single()? } else { 6u64 };
    let count = if !arg.is_empty() { arg.single()? } else { 1u64 };
    if !(1..=DICE_MAX).contains(&d) || !(1..=DICE_COUNT_MAX).contains(&count) {
        msg.reply(
            ctx,
            format!("Invalid parameter\n1 <= dice <= {DICE_MAX}, 1 <= count <= {DICE_COUNT_MAX}"),
        )
        .await?;
        return Ok(());
    }

    let nums = dice_core(d, count);
    let nums: Vec<_> = nums.iter().map(|n| n.to_string()).collect();
    let buf = nums.join(", ");
    assert!(!buf.is_empty());

    msg.reply(ctx, buf).await?;

    Ok(())
}

#[command]
#[description("Delete messages other than the most recent N ones.")]
#[usage("<N>")]
#[example("100")]
#[num_args(1)]
async fn delmsg(ctx: &Context, msg: &Message, mut arg: Args) -> CommandResult {
    owner_check(ctx, msg).await?;

    let n: usize = arg.single()?;

    // id 昇順で後ろ n 個を残して他を消す
    let (delcount, total) =
        delete_msgs_in_channel(ctx, msg.channel_id, |_m, i, len| i + n < len).await?;

    msg.reply(ctx, format!("{delcount}/{total} messages deleted"))
        .await?;

    Ok(())
}

#[command]
#[description("Take a picture.")]
#[usage("")]
#[example("")]
async fn camera(ctx: &Context, msg: &Message) -> CommandResult {
    owner_check(ctx, msg).await?;

    let pic = take_a_pic(TakePicOption::new()).await?;
    let attachment = CreateAttachment::bytes(&pic[..], "camera.jpg");
    let cm = CreateMessage::new().add_file(attachment);
    msg.channel_id.send_message(ctx, cm).await?;

    Ok(())
}

#[command]
#[description("Let me say a message to the specified user.")]
#[usage("here <user> <msg>")]
#[usage("<channel> <user> <msg>")]
#[example("12345 6789 hello")]
#[num_args(3)]
async fn attack(ctx: &Context, msg: &Message, mut arg: Args) -> CommandResult {
    owner_check(ctx, msg).await?;

    let chstr: String = arg.single()?;
    let ch = if chstr == "here" {
        msg.channel_id
    } else {
        ChannelId::new(chstr.parse::<u64>()?)
    };
    let user = UserId::new(arg.single::<u64>()?);
    let text: String = arg.single_quoted()?;

    let cm = CreateMessage::new().content(format!("{} {}", user.mention(), text));
    ch.send_message(ctx, cm).await?;

    Ok(())
}

#[command]
#[description("OpenAI chat assistant.")]
#[usage("<your message>")]
#[example("Hello, what's your name?")]
#[min_args(1)]
async fn ai(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
    let chat_msg = arg.rest();

    let data = ctx.data.read().await;
    let ctrl = data.get::<ControlData>().unwrap();
    let mut discord = ctrl.sysmods().discord.lock().await;
    let config = data.get::<ConfigData>().unwrap();

    // タイムアウト処理
    discord.check_history_timeout();

    // 今回の発言をヒストリに追加 (システムメッセージ + 本体)
    let sysmsg = config
        .prompt
        .each
        .join("")
        .replace("${user}", &msg.author.name);
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
        let ai = ctrl.sysmods().openai.lock().await;

        // 送信用リスト
        let mut all_msgs = Vec::new();
        // 先頭システムメッセージ
        all_msgs.push(ChatMessage {
            role: Role::System,
            content: Some(config.prompt.pre.join("")),
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
            reply_long(msg, ctx, &text).await?;

            // タイムアウト延長
            discord.chat_timeout = Some(
                Instant::now() + Duration::from_secs(config.prompt.history_timeout_min as u64 * 60),
            );
        }
        Err(err) => {
            error!("[discord] openai error: {:#?}", err);
            // HTTP status が得られるタイプのエラーのみ discord 返信する
            if let Some(err) = err.downcast_ref::<HttpStatusError>() {
                warn!("openai reply: {} {}", err.status, err.body);
                let reply_msg = format!("OpenAI API Error, HTTP Status: {}", err.status);
                msg.reply(ctx, reply_msg.to_string()).await?;
            }
        }
    }

    Ok(())
}

#[command]
#[description("Get ai command status.")]
#[usage("")]
#[example("")]
async fn aistatus(ctx: &Context, msg: &Message) -> CommandResult {
    let text = {
        let data = ctx.data.read().await;
        let ctrl = data.get::<ControlData>().unwrap();
        let mut discord = ctrl.sysmods().discord.lock().await;
        let config = data.get::<ConfigData>().unwrap();

        discord.check_history_timeout();
        format!(
            "History: {}\nToken: {} / {}, Timeout: {} min",
            discord.chat_history.len(),
            discord.chat_history.usage().0,
            discord.chat_history.usage().1,
            config.prompt.history_timeout_min
        )
    };
    msg.reply(ctx, text).await?;

    Ok(())
}

#[command]
#[description("Clear ai command history.")]
#[usage("")]
#[example("")]
async fn aireset(ctx: &Context, msg: &Message) -> CommandResult {
    {
        let data = ctx.data.read().await;
        let ctrl = data.get::<ControlData>().unwrap();
        let mut discord = ctrl.sysmods().discord.lock().await;

        discord.chat_history.clear();
    }
    msg.reply(ctx, "OK").await?;

    Ok(())
}

#[command]
#[description("OpenAI image generation.")]
#[usage("<prompt>")]
#[example("A person who are returning home early from their office.")]
#[min_args(1)]
async fn aiimg(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
    let prompt = arg.rest();

    let img_url = {
        let data = ctx.data.read().await;
        let ctrl = data.get::<ControlData>().unwrap();
        let ai = ctrl.sysmods().openai.lock().await;

        let mut resp = ai.generate_image(prompt, 1).await?;
        resp.pop().ok_or_else(|| anyhow!("image array too short"))?
    };

    msg.reply(ctx, img_url).await?;

    Ok(())
}

#[group]
#[prefix = "autodel"]
#[commands(status, set)]
struct AutoDel;

const INVALID_CH_MSG: &str = "Auto delete feature is not enabled for this channel.
Please contact my owner.";

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

#[command]
#[description("Get the auto-delete feature status in this channel.")]
#[usage("")]
#[example("")]
async fn status(ctx: &Context, msg: &Message) -> CommandResult {
    let ch = msg.channel_id;
    let config = {
        let data = ctx.data.read().await;
        let ctrl = data.get::<ControlData>().unwrap();
        let discord = ctrl.sysmods().discord.lock().await;

        discord.auto_del_config.get(&ch).copied()
    };

    if let Some(config) = config {
        msg.reply(ctx, format!("{config}")).await?;
    } else {
        msg.reply(ctx, INVALID_CH_MSG).await?;
    }

    Ok(())
}

#[command]
#[description(
    r#"Enable/Disable/Config auto-delete feature in this channel.
"0 0" disables the feature."#
)]
#[usage("<keep_count> <keep_duration>")]
#[example("0 0")]
#[example("100 1d")]
#[example("200 12h")]
#[example("300 1d23h59m")]
#[num_args(2)]
// disabled in Direct Message
#[only_in("guild")]
async fn set(ctx: &Context, msg: &Message, mut arg: Args) -> CommandResult {
    let keep_count: u32 = arg.single()?;
    let keep_duration: String = arg.single()?;
    let keep_duration: u32 = parse_duration(&keep_duration)?;
    let ch = msg.channel_id;

    {
        let data = ctx.data.read().await;
        let ctrl = data.get::<ControlData>().unwrap();
        let mut discord = ctrl.sysmods().discord.lock().await;

        let config = discord.auto_del_config.get_mut(&ch);
        match config {
            Some(config) => {
                config.keep_count = keep_count;
                config.keep_dur_min = keep_duration;
                msg.reply(ctx, format!("OK\n{config}")).await?;
            }
            None => {
                msg.reply(ctx, INVALID_CH_MSG).await?;
            }
        }
    };

    Ok(())
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
    fn dice_6_many_times() {
        let mut result = dice_core(6, DICE_COUNT_MAX);
        assert_eq!(result.len(), DICE_COUNT_MAX as usize);

        // 100 回も振れば 1..=6 が 1 回ずつは出る
        result.sort();
        for x in 1..=6 {
            assert!(result.binary_search(&x).is_ok());
        }
    }

    #[test]
    #[should_panic]
    fn dice_invalid_dice() {
        let _ = dice_core(DICE_MAX + 1, 1);
    }

    #[test]
    #[should_panic]
    fn dice_invalid_count() {
        let _ = dice_core(6, DICE_COUNT_MAX + 1);
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
