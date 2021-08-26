mod commands;
mod util;

use std::{collections::HashSet, env};

use mongodb::{Collection, Database, bson::doc};
use serde_json::json;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    prelude::TypeMapKey,
    Client,
};

use commands::{ping::*, tunnel::*};
use util::u64_to_i64;

use serde::{Deserialize, Serialize};

#[group]
#[commands(ping)]
struct General;

#[group]
#[prefixes("tunnel")]
#[owners_only]
#[only_in(guilds)]
#[commands(link, unlink, set_category)]
struct Tunnel;

#[derive(Debug, Deserialize, Serialize)]
struct TunnelInfo {
    other_channel_id: u64,
    thunderstore_channel_id: u64,
    other_webhook_url: String,
    thunderstore_webhook_url: String,
}

struct TunnelsCollection;

impl TypeMapKey for TunnelsCollection {
    type Value = Collection<TunnelInfo>;
}

struct DatabaseContainer;

impl TypeMapKey for DatabaseContainer {
    type Value = Database;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BotConfig {
    thunderstore_guild_id: u64,
    thunderstore_category_id: u64,
}

impl TypeMapKey for BotConfig {
    type Value = Option<BotConfig>;
}

struct NetClient;
impl TypeMapKey for NetClient {
    type Value = reqwest::Client;
}

struct TunnelHandler;

#[async_trait]
impl EventHandler for TunnelHandler {
    async fn ready(&self, _: Context, _: serenity::model::prelude::Ready) {
        println!("Connected to gateway");
    }

    async fn message(
        &self,
        ctx: Context,
        msg: serenity::model::channel::Message,
    ) {
        if msg.author.bot {
            return;
        }
        if msg.content.starts_with("~") {
            return;
        }

        let id = msg.channel_id.0;

        let typemap = ctx.data.read().await;
        let tunnels = typemap.get::<TunnelsCollection>().unwrap();
        let client = typemap.get::<NetClient>().unwrap();

        let unsafe_id = u64_to_i64(id);

        if let Some(tunnel) = tunnels.find_one(doc! { "$or": [ { "thunderstore_channel_id":unsafe_id }, { "other_channel_id":unsafe_id } ] }, None).await.unwrap()
        {
            let url = if id == tunnel.thunderstore_channel_id {
                &tunnel.other_webhook_url
            } else {
                &tunnel.thunderstore_webhook_url
            };

            client.post(url)
                .json(&json!({
                    "content": msg.content_safe(&ctx).await,
                    "username": format!("{}#{}", msg.author.name, msg.author.discriminator),
                    "avatar_url": msg.author.avatar_url().or(Some(msg.author.default_avatar_url())).unwrap()
                }))
                .send()
                .await
                .expect("Error sending webhook message");
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
    let token = env::var("DISCORD_TOKEN").expect("Expected a Discord bot token in env var DISCORD_TOKEN");

    let client = mongodb::Client::with_uri_str("mongodb://root:password@mongo:27017").await.unwrap();
    let db = client.database("test");

    let config = db.collection::<BotConfig>("settings").find_one(None, None).await.unwrap();
    if config.is_none() {
        println!("No settings object found, please use `~tunnel set_category` as your first command");
    }

    let http = Http::new_with_token(&token);

    let owners = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();

            if let Some(team) = info.team {
                for owner in team.members {
                    owners.insert(owner.user.id);
                }
            } else {
                owners.insert(info.owner.id);
            }

            owners
        }
        Err(e) => {
            panic!("Couldn't access app info: {:?}", e);
        }
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("~"))
        .group(&GENERAL_GROUP)
        .group(&TUNNEL_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(TunnelHandler)
        .await
        .expect("Failed to create client");

    let mut data = client.data.write().await;
    data.insert::<TunnelsCollection>(db.collection("tunnels"));
    data.insert::<BotConfig>(config);
    data.insert::<NetClient>(reqwest::Client::default());
    data.insert::<DatabaseContainer>(db);
    drop(data);

    let shard_man = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to register ctrl-c handler");
        shard_man.lock().await.shutdown_all().await;
    });

    println!("Finished loading");

    if let Err(e) = client.start().await {
        panic!("Failed to start client: {:?}", e);
    }
}
