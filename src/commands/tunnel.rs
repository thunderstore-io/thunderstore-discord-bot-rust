use reqwest::StatusCode;
use serde_json::json;
use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};
use mongodb::bson::doc;

use crate::{BotConfig, DatabaseContainer, TunnelInfo, TunnelsCollection, NetClient};
use crate::util::u64_to_i64;

#[command]
async fn link(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let config = data.get::<BotConfig>().unwrap().as_ref().expect("No config object, please run `~tunnel set_category");

    let tstore_channel = ctx
        .http
        .create_channel(
            config.thunderstore_guild_id,
            json!({
                "name": msg.guild(ctx).await.unwrap().name,
                "type": 0,
                "parent_id": config.thunderstore_category_id
            })
            .as_object()
            .unwrap(),
        )
        .await?;

    let tstore_webhook = ctx
        .http
        .create_webhook(
            *tstore_channel.id.as_u64(),
            &json!({
                "name": "Thunderstore Tunnel Webhook"
            }),
        )
        .await?;

    let other_webhook = ctx
        .http
        .create_webhook(
            *msg.channel_id.as_u64(),
            &json!({
                "name": "Thunderstore Tunnel Webhook"
            }),
        )
        .await?;

    let tunnels = data.get::<TunnelsCollection>().unwrap();

    tunnels.insert_one(TunnelInfo { 
        thunderstore_channel_id: tstore_channel.id.0,
        other_channel_id: msg.channel_id.0,
        thunderstore_webhook_url: tstore_webhook.url().unwrap(),
        other_webhook_url: other_webhook.url().unwrap(),
     }, None).await?;

    msg.channel_id
        .say(&ctx, "Finished linking channels")
        .await?;

    Ok(())
}

#[command]
async fn unlink(ctx: &Context, msg: &Message) -> CommandResult {
    let id = msg.channel_id.0;

    let data = ctx.data.read().await;

    let tunnels = data.get::<TunnelsCollection>().unwrap();
    let client = data.get::<NetClient>().unwrap();

    let unsafe_id = u64_to_i64(id);

    let tunnel_opt = tunnels.find_one_and_delete(doc! { "$or": [ { "thunderstore_channel_id":unsafe_id }, { "other_channel_id":unsafe_id } ] }, None).await?;

    match tunnel_opt {
        Some(tunnel) => {
            if msg.content.contains("true") {
                ctx.http.delete_channel(tunnel.thunderstore_channel_id).await?;
            }
            else {
                let res = client.delete(tunnel.thunderstore_webhook_url).send().await?;
                if res.status() != StatusCode::NO_CONTENT {
                    msg.channel_id.say(ctx, "Failed to delete the thunderstore webhook").await?;
                }
            }

            let res = client.delete(tunnel.other_webhook_url).send().await?;
            if res.status() != StatusCode::NO_CONTENT {
                msg.channel_id.say(ctx, "Failed to delete this server's webhook, please delete it manually").await?;
            }

            Ok(())
        },
        None => Ok(())
    }
}

#[command]
async fn set_category(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;

    let parts: Vec<&str> = msg.content.split(" ").collect();
    if parts.len() != 4 {
        msg.channel_id.say(ctx, "Invalid arg length").await?;
    }

    let config = BotConfig {
        thunderstore_guild_id: u64::from_str_radix(parts[2], 10).expect("Invalid guild ID in args[2]"),
        thunderstore_category_id: u64::from_str_radix(parts[3], 10).expect("Invalid category ID in args[3]")
    };

    let db = data.get::<DatabaseContainer>().unwrap();
    db.collection("settings").insert_one(config.clone(), None).await?;
    data.remove::<BotConfig>();
    data.insert::<BotConfig>(Some(config.clone()));

    msg.channel_id.say(ctx, format!("{:?}", config)).await?;

    Ok(())
}
