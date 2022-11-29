use std::collections::HashSet;

use super::SystemModule;
use crate::sys::{config, taskserver::Control};
use anyhow::{anyhow, Result};
use log::info;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group, help};
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandResult, HelpOptions,
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
        .configure(|c| c.prefix("").on_mention(Some(UserId(info.id.0)))) // set the bot's prefix to "~"
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

#[command]
async fn dice(ctx: &Context, msg: &Message, mut arg: Args) -> CommandResult {
    const DICE_MAX: u64 = 1u64 << 56;
    const COUNT_MAX: u64 = 100u64;
    const_assert!(DICE_MAX < u64::MAX / COUNT_MAX);

    let d = if !arg.is_empty() { arg.single()? } else { 6u64 };
    if !(1..=DICE_MAX).contains(&d) {
        msg.reply(ctx, format!("Invalid dice: {}", d)).await?;
        return Ok(());
    }
    let count = if !arg.is_empty() { arg.single()? } else { 1u64 };
    if !(1..=COUNT_MAX).contains(&count) {
        msg.reply(ctx, format!("Invalid count: {}", count)).await?;
        return Ok(());
    }

    let mut buf = String::new();
    {
        // ThreadRng はスレッド間移動できないので await をまたげない
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            if !buf.is_empty() {
                buf.push_str(", ");
            }
            buf.push_str(&rng.gen_range(1..=DICE_MAX).to_string());
        }
    }
    assert!(!buf.is_empty());

    msg.reply(ctx, buf).await?;

    Ok(())
}

#[command]
async fn delmsg(ctx: &Context, msg: &Message) -> CommandResult {
    ctx.http.delete_message(msg.channel_id.0, msg.id.0).await?;

    Ok(())
}
