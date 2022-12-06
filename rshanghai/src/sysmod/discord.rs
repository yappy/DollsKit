//! Discord クライアント (bot) 機能。

use super::camera::{take_a_pic, TakePicOption};
use super::SystemModule;
use crate::sys::version::VERSION_INFO;
use crate::sys::{config, taskserver::Control};
use anyhow::{anyhow, Result};
use log::{error, info, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group, help, hook};
use serenity::framework::standard::{
    help_commands, Args, CommandError, CommandGroup, CommandResult, HelpOptions,
};
use serenity::http::Http;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{framework::StandardFramework, Client};
use static_assertions::const_assert;
use std::collections::{BTreeSet, HashSet};

/// Discord 設定データ。json 設定に対応する。
#[derive(Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// 機能を有効化するなら true。
    enabled: bool,
    /// アクセストークン。Developer Portal で入手できる。
    token: String,
    /// メッセージの発言先チャネル。
    /// Discord の詳細設定で開発者モードを有効にして、チャネルを右クリックで
    /// ID をコピーできる。
    notif_channel: u64,
    /// パーミッションエラーメッセージ。
    /// オーナーのみ使用可能なコマンドを実行しようとした。
    perm_err_msg: String,
    /// パーミッションエラーを強制的に発生させる。デバッグ用。
    force_perm_err: bool,
}

/// Discord システムモジュール。
pub struct Discord {
    /// 設定データ。
    config: DiscordConfig,
    /// 現在有効な Discord Client コンテキスト。
    ///
    /// 起動直後は None で、[Handler::cache_ready] イベントの度に置き換わる。
    ctx: Option<Context>,
}

impl Discord {
    /// コンストラクタ。
    ///
    /// 設定データの読み込みのみ行い、実際の初期化は async が有効になる
    /// [discord_main] で行う。
    pub fn new() -> Result<Self> {
        info!("[discord] initialize");

        let jsobj = config::get_object(&["discord"])
            .map_or(Err(anyhow!("Config not found: discord")), Ok)?;
        let config: DiscordConfig = serde_json::from_value(jsobj)?;

        Ok(Self { config, ctx: None })
    }

    pub async fn say(&mut self, msg: &str) -> Result<()> {
        if !self.config.enabled {
            info!("discord disabled - msg: {}", msg);
            return Ok(());
        }
        if self.config.notif_channel == 0 {
            info!("discord notification disabled - msg: {}", msg);
            return Ok(());
        }
        if self.ctx.is_none() {
            warn!("discord not ready - msg: {}", msg);
            return Ok(());
        }

        info!("discord say msg: {}", msg);
        let ch = ChannelId(self.config.notif_channel);
        let ctx = self.ctx.as_ref().unwrap();
        ch.say(ctx, msg).await?;

        Ok(())
    }
}

/// システムを初期化し開始する。
///
/// [Discord::on_start] から spawn される。
async fn discord_main(ctrl: Control) -> Result<()> {
    let config = {
        let discord = ctrl.sysmods().discord.lock().await;
        discord.config.clone()
    };

    // 自身の ID が on_mention 設定に必要なので別口で取得しておく
    let http = Http::new(&config.token);
    let info = http.get_current_application_info().await?;
    let myid = UserId(info.id.0);
    let ownerids = HashSet::from([info.owner.id]);

    let framework = StandardFramework::new()
        // コマンドのプレフィクスはなし
        // bot へのメンションをトリガとする
        .configure(|c| c.prefix("").on_mention(Some(myid)).owners(ownerids.clone()))
        // コマンド前後でのフック (ロギング用)
        .before(before_hook)
        .after(after_hook)
        .unrecognised_command(unrecognised_hook)
        // コマンドとヘルプの登録
        .group(&GENERAL_GROUP)
        .help(&MY_HELP);

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
        shard_manager.lock().await.shutdown_all().await;
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

    // システムスタート
    client.start().await?;
    info!("[discord] client exit");

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
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        info!("[discord] cache ready - guild: {}", guilds.len());

        let ctx_clone = ctx.clone();
        let ctx = ctx.data.read().await;
        let ctrl = ctx.get::<ControlData>().unwrap();
        let mut discord = ctrl.sysmods().discord.lock().await;
        discord.ctx = Some(ctx_clone);
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
#[commands(sysinfo, dice, delmsg, camera, attack)]
struct General;

#[command]
#[description("Print system information.")]
#[usage("")]
#[example("")]
async fn sysinfo(ctx: &Context, msg: &Message) -> CommandResult {
    let ver_info: &str = &*VERSION_INFO;
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
            format!(
                "Invalid parameter\n1 <= dice <= {}, 1 <= count <= {}",
                DICE_MAX, DICE_COUNT_MAX
            ),
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

    let n: u32 = arg.single()?;

    // id=0 から 100 件ずつすべてのメッセージを取得する
    let mut allmsgs = BTreeSet::<u64>::new();
    const GET_MSG_LIMIT: usize = 100;
    let mut after = 0u64;
    loop {
        // https://discord.com/developers/docs/resources/channel#get-channel-messages
        let query = format!("?after={}&limit={}", after, GET_MSG_LIMIT);
        info!("get_messages: {}", query);
        let msgs = ctx.http.get_messages(msg.channel_id.0, &query).await;
        let msgs = msgs?;

        // 空配列ならば完了
        if msgs.is_empty() {
            break;
        }
        // message id を取り出してセットに追加する
        // 降順で送られてくるのでソートし直す
        allmsgs.extend(msgs.iter().map(|m| m.id.0));
        // 最後の message id を次回の after に設定する
        after = *allmsgs.iter().next_back().unwrap();
    }
    info!("obtained {} messages", allmsgs.len());

    // id 昇順で後ろ n 個を残して他を消す
    if allmsgs.len() <= n as usize {
        msg.reply(ctx, format!("0/{} messages deleted", allmsgs.len()))
            .await?;

        return Ok(());
    }
    let delcount = allmsgs.len() - n as usize;

    // Bulk delete 機能で一気に複数を消せるが、2週間以上前のメッセージが
    // 含まれていると BAD REQUEST になる等扱いが難しいので rate limit は
    // 気になるが1つずつ消す
    info!("Delete {} messages...", delcount);
    for &mid in allmsgs.iter().take(delcount) {
        // ch, msg はログに残す
        info!("Delete: ch={}, msg={}", msg.channel_id.0, mid);
        // https://discord.com/developers/docs/resources/channel#delete-message
        msg.channel_id.delete_message(ctx, mid).await?;
    }

    msg.reply(
        ctx,
        format!("{}/{} messages deleted", delcount, allmsgs.len()),
    )
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
    msg.channel_id
        .send_message(ctx, |m| m.add_file((&pic[..], "camera.jpg")))
        .await?;

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
        ChannelId(chstr.parse::<u64>()?)
    };
    let user = UserId(arg.single::<u64>()?);
    let text: String = arg.single_quoted()?;

    ch.send_message(ctx, |m| {
        m.content(format_args!("{} {}", user.mention(), text))
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
