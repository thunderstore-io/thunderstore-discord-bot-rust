use serde_json::json;
use serenity::{client::Context, framework::standard::{CommandResult, macros::command}, model::channel::Message};

use crate::{BotConfig, TunnelInfo, TunnelsContainer};

#[command]
async fn link(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let config = data.get::<BotConfig>().unwrap();

    let tstore_channel = ctx.http.create_channel(config.thunderstore_guild_id, json!({
        "name": msg.guild(ctx).await.unwrap().name,
        "type": 0,
        "parent_id": config.thunderstore_category_id
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

    let tunnels = data.get_mut::<TunnelsContainer>().unwrap();
    
    tunnels.push(TunnelInfo {
        thunderstore_channel_id: *tstore_channel.id.as_u64(),
        thunderstore_webhook_url: tstore_webhook.url().unwrap(),
        other_channel_id: *msg.channel_id.as_u64(),
        other_webhook_url: other_webhook.url().unwrap()
    });

    std::fs::write("cache/tunnels.json", serde_json::to_string_pretty(tunnels).unwrap())?;

    msg.channel_id.say(&ctx, "Finished linking channels").await?;

    Ok(())
}

#[command]
async fn unlink(ctx: &Context, msg: &Message) -> CommandResult {
    let id = *msg.channel_id.as_u64();
    
    let mut data = ctx.data.write().await;

    let tunnels = data.get_mut::<TunnelsContainer>().unwrap();

    tunnels.retain(|x| x.thunderstore_channel_id != id && x.other_channel_id != id);

    std::fs::write("cache/tunnels.json", serde_json::to_string_pretty(tunnels).unwrap())?;

    msg.channel_id.say(&ctx, "Finished unlinking channels (the channels and webhooks still exist, please remove them manually)").await?;

    Ok(())
}
