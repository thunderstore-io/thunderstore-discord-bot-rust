mod commands;

use std::{collections::HashSet, env, io::Read};

use serde_json::json;
use serenity::{Client, client::EventHandler, framework::{StandardFramework, standard::macros::{group}}, http::Http, prelude::TypeMapKey, async_trait};

use commands::{ping::*, tunnel::*};

use serde::{Deserialize, Serialize};

#[group]
#[commands(ping)]
struct General;

#[group]
#[prefixes("tunnel")]
#[owners_only]
#[only_in(guilds)]
#[commands(link, unlink)]
struct Tunnel;

#[derive(Deserialize, Serialize)]
struct TunnelInfo {
    other_channel_id: u64,
    thunderstore_channel_id: u64,
    other_webhook_url: String,
    thunderstore_webhook_url: String,
}

struct TunnelsContainer;

impl TypeMapKey for TunnelsContainer {
    type Value = Vec<TunnelInfo>;
}

struct TunnelHandler;

#[async_trait]
impl EventHandler for TunnelHandler {
    async fn message(&self, ctx: serenity::client::Context, msg: serenity::model::channel::Message) {
        if msg.author.bot { return; }
        
        let id = *msg.channel_id.as_u64();

        let typemap = ctx.data.read().await;
        let tunnels = typemap.get::<TunnelsContainer>().unwrap();

        if let Some(tunnel) = tunnels.iter().find(|x| x.other_channel_id == id || x.thunderstore_channel_id == id) {
            let url = if id == tunnel.thunderstore_channel_id { &tunnel.other_webhook_url } else { &tunnel.thunderstore_webhook_url };

            reqwest::Client::default().post(url)
                .json(&json!({
                    "content": msg.content_safe(&ctx).await,
                    "username": msg.author.name,
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
    let token = env::var("DISCORD_TOKEN").expect("Expected a discord token in env");

    let http = Http::new_with_token(&token);

    let owners = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            
            if let Some(team) = info.team {
                for owner in team.members {
                    owners.insert(owner.user.id);
                }
            }
            else {
                owners.insert(info.owner.id);
            }

            owners
        },
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

    if std::fs::read_dir("cache").is_err() { std::fs::create_dir("cache").expect("Failed to create cache path") }

    let tunnels: Vec<TunnelInfo> = match std::fs::File::open("cache/tunnels.json") {
        Ok(mut file) => {
            let mut tunnels_json = String::new();
            file.read_to_string(&mut tunnels_json).expect("Error reading from permanent tunnel file");

            serde_json::from_str(&tunnels_json).expect("Error deserializing permanent tunnel json")
        },
        _ => {
            Vec::new()
        }
    };

    let mut data = client.data.write().await;
    data.insert::<TunnelsContainer>(tunnels);
    drop(data);

    let shard_man = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to register ctrl-c handler");
        shard_man.lock().await.shutdown_all().await;
    });

    if let Err(e) = client.start().await {
        panic!("Failed to start client: {:?}", e);
    }
}
