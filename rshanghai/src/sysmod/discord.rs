use super::SystemModule;
use crate::sys::{config, taskserver::Control};
use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::CommandResult;
use serenity::http::Http;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{framework::StandardFramework, Client};

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
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    let mut ctrl_for_cancel = ctrl.clone();
    let shard_manager = client.shard_manager.clone();
    ctrl.spawn_oneshot_fn("discord-cancel", async move {
        ctrl_for_cancel.cancel_rx().changed().await.unwrap();
        info!("[discord-cancel] recv cancel");
        shard_manager.lock().await.shutdown_all().await;
        info!("[discord-cancel] shutdown_all ok");

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

#[group]
#[commands(ping, delmsg)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("[discord] connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("[discord] resumed");
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}

#[command]
async fn delmsg(ctx: &Context, msg: &Message) -> CommandResult {
    ctx.http.delete_message(msg.channel_id.0, msg.id.0).await?;

    Ok(())
}
