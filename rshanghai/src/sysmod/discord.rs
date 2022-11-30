use std::collections::{BTreeSet, HashSet};

use super::SystemModule;
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

#[derive(Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    enabled: bool,
    token: String,
}

pub struct Discord {
    config: DiscordConfig,
}

impl Discord {
    pub fn new() -> Result<Self> {
        info!("[discord] initialize");

        let jsobj = config::get_object(&["discord"])
            .map_or(Err(anyhow!("Config not found: discord")), Ok)?;
        let config: DiscordConfig = serde_json::from_value(jsobj)?;

        Ok(Self { config })
    }
}

async fn discord_main(ctrl: Control) -> Result<()> {
    let discord = ctrl.sysmods().discord.lock().await;
    let token = discord.config.token.clone();
    drop(discord);

    let http = Http::new(&token);
    let info = http.get_current_application_info().await?;

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("").on_mention(Some(UserId(info.id.0))))
        .before(before_hook)
        .after(after_hook)
        .unrecognised_command(unrecognised_hook) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP)
        .help(&MY_HELP);

    // Login with a bot token from the environment
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    let mut ctrl_for_cancel = ctrl.clone();
    let shard_manager = client.shard_manager.clone();
    ctrl.spawn_oneshot_fn("discord-exit", async move {
        ctrl_for_cancel.cancel_rx().changed().await.unwrap();
        info!("[discord-exit] recv cancel");
        shard_manager.lock().await.shutdown_all().await;
        info!("[discord-exit] shutdown_all ok");

        Ok(())
    });

    // start listening for events by starting a single shard
    client.start().await?;
    info!("[discord] client exit");

    Ok(())
}

#[hook]
async fn before_hook(_: &Context, msg: &Message, cmd_name: &str) -> bool {
    info!("[discord] command {} by {}", cmd_name, msg.author.name);

    true
}

#[hook]
async fn after_hook(_: &Context, _: &Message, cmd_name: &str, result: Result<(), CommandError>) {
    if let Err(why) = result {
        error!("[discord] error in {}: {:?}", cmd_name, why);
    }
}

#[hook]
async fn unrecognised_hook(_: &Context, msg: &Message, unrecognised_command_name: &str) {
    warn!(
        "[discord] unknown command {} by {}",
        unrecognised_command_name, msg.author.name
    );
}

impl SystemModule for Discord {
    fn on_start(&self, ctrl: &Control) {
        info!("[discord] on_start");
        if self.config.enabled {
            ctrl.spawn_oneshot_task("discord", discord_main);
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("[discord] connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("[discord] resumed");
    }

    async fn cache_ready(&self, _ctx: Context, guilds: Vec<GuildId>) {
        info!("[discord] cache ready - guild: {}", guilds.len());
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

#[group]
#[commands(dice, delmsg)]
struct General;

const DICE_MAX: u64 = 1u64 << 56;
const DICE_COUNT_MAX: u64 = 100u64;
const_assert!(DICE_MAX < u64::MAX / DICE_COUNT_MAX);

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
#[description("!!! Implementation Incomplete !!!")]
#[usage("N")]
#[example("100")]
#[num_args(1)]
async fn delmsg(ctx: &Context, msg: &Message, mut arg: Args) -> CommandResult {
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
