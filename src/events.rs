use serenity::all::ChannelId;
use serenity::all::ComponentInteractionDataKind;
use serenity::all::CreateMessage;
use serenity::all::Interaction;
use serenity::all::Ready;
use serenity::all::VoiceState;
use serenity::prelude::*;
use serenity::async_trait;

use crate::botdata::BotDataKey;
use crate::commands;
use crate::vote;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, botdata: Ready) {
        log::info!("Logged in as {}", botdata.user.name);
        if let Err(err) = commands::register_commands(&ctx.http).await {
            log::error!("Unable to register commands: {}", err.to_string())
        };
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(ref command) => commands::handle_commands(ctx, command).await,
            Interaction::Component(ref component_interaction) => {
                if let ComponentInteractionDataKind::Button = component_interaction.data.kind {
                    if component_interaction.data.custom_id.starts_with("vote") {
                        vote::handle_voting(ctx, component_interaction).await;
                    }
                }
            },
            _ => (),
        }
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        // Do we have a member?
        if let Some(ref member) = new.member {
            // If bot disconnected
            if ctx.cache.current_user().id == member.user.id && new.channel_id.is_none() {
                on_disconnect(&ctx, &_old, &new).await;
            }
        }
        if new.guild_id.is_some() {
            disconnect_if_nobody(&ctx, &_old, &new).await;
        }
    }
}

async fn disconnect_if_nobody(ctx: &Context, _old: &Option<VoiceState>, new: &VoiceState) {
    let guild_id = new.guild_id.unwrap();

    let voice_channel_id: ChannelId;
    let call_u = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .get(guild_id);
    if let Some(call) = call_u {
        if call.lock().await
        .current_channel().is_none() {
            return;
        }

        voice_channel_id = ChannelId::from(
            call.lock().await
            .current_channel().unwrap()
            .0
        );
    } else { return; }

    // Get VC members and subtract by one because the bot shouldn't count as a member
    let members_count = ctx.cache.guild(guild_id).unwrap()
        .channels.get(&voice_channel_id).unwrap()
        .members(&ctx).unwrap()
        .len()
        .saturating_sub(1);

    if members_count != 0 { return; }

    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&guild_id);
    if session_u.is_none() {
        return;
    }

    let session = session_u.unwrap().clone();
    let session_lock = session.data.read().await;

    let text_channel_id = session_lock.text_channel_id;
    drop(session_lock);
    drop(data_lock);

    crate::misc::leave_vc(&ctx, guild_id).await.unwrap();
    text_channel_id.send_message(&ctx.http, CreateMessage::new().content("No users in the voice channel, leaving")).await.unwrap();
}

async fn on_disconnect(ctx: &Context, _old: &Option<VoiceState>, new: &VoiceState) {
    if new.guild_id.is_none() { return };
    let guild_id = new.guild_id.unwrap();

    crate::misc::remove_session(&ctx, guild_id).await;
}