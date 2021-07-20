use serde_json::json;
use serenity::{client::Context, framework::standard::{CommandResult, macros::command}, model::channel::Message};

use crate::{TunnelInfo, TunnelsContainer};

const THUNDERSTORE_GUILD_ID: u64 = 335090384055042060;
const TUNNEL_CATEGORY_ID: u64 = 866896866377072670;

#[command]
async fn link(ctx: &Context, msg: &Message) -> CommandResult {
    let tstore_channel = ctx.http.create_channel(THUNDERSTORE_GUILD_ID, json!({
        "name": msg.guild(ctx).await.unwrap().name,
        "type": 0,
        "parent_id": TUNNEL_CATEGORY_ID
    }).as_object().unwrap())
    .await?;

    let tstore_webhook = ctx.http.create_webhook(*tstore_channel.id.as_u64(), &json!({
        "name": "Thunderstore Tunnel Webhook"
    }))
    .await?;

    let other_webhook = ctx.http.create_webhook(*msg.channel_id.as_u64(), &json!({
        "name": "Thunderstore Tunnel Webhook"
    }))
    .await?;

    let mut data = ctx.data.write().await;
    let tunnels = data.get_mut::<TunnelsContainer>().unwrap();
    
    tunnels.push(TunnelInfo {
        thunderstore_channel_id: *tstore_channel.id.as_u64(),
        thunderstore_webhook_url: tstore_webhook.url().unwrap(),
        other_channel_id: *msg.channel_id.as_u64(),
        other_webhook_url: other_webhook.url().unwrap()
    });

    std::fs::write("cache/tunnels.json", serde_json::to_string_pretty(tunnels).unwrap())?;

    Ok(())
}

#[command]
async fn unlink(ctx: &Context, msg: &Message) -> CommandResult {
    let id = *msg.channel_id.as_u64();
    
    let mut data = ctx.data.write().await;

    let tunnels = data.get_mut::<TunnelsContainer>().unwrap();

    tunnels.retain(|x| x.thunderstore_channel_id != id && x.other_channel_id != id);

    std::fs::write("cache/tunnels.json", serde_json::to_string_pretty(tunnels).unwrap())?;

    Ok(())
}
