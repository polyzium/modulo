use serenity::all::{ChannelId, ChannelType, CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::builder::CreateCommand;
use songbird::input::RawAdapter;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;
use crate::session::VoiceSession;

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let (guild_id, voice_channel_id) = {
        let guild_id = interaction.guild_id.unwrap();
        let voice_channel_id: Option<ChannelId> = {
            let channels = guild_id
            .to_partial_guild(&ctx.http).await.unwrap()
            .channels(&ctx.http).await.unwrap();

            let channel = channels.values()
                .find(|channel| {
                    if channel.kind != ChannelType::Voice { return false };
                    let members = channel.members(&ctx).unwrap();
                    let member = members
                        .iter()
                        .find(|member| member.user.id == interaction.user.id);
                    member.is_some()
                });
            match channel {
                Some(channel) => Some(channel.id),
                None => None,
            }
        };

        (guild_id, voice_channel_id)
    };

    {
        let mut lock = ctx.data.write().await;
        let botdata = lock.get_mut::<BotDataKey>().unwrap();
        if let Some(_) = botdata.sessions.get(&guild_id) {
            respond_command(&ctx, interaction, "The bot is already in a voice channel").await;
            return;
        }
    }

    let connect_to = match voice_channel_id {
        Some(channel) => channel,
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                .content("Join a voice channel first".to_string())
            );
            interaction.create_response(ctx.http, response).await.unwrap();
            return;
        },
    };

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    match manager.join(guild_id, connect_to).await {
        Ok(handler_lock) => {
            let mut handler = handler_lock.lock().await;

            let session = VoiceSession::new(&ctx, interaction.channel_id);
            let voicedata = session.data.clone();

            let mut lock = ctx.data.write().await;
            let botdata = lock.get_mut::<BotDataKey>().unwrap();
            botdata.sessions.insert(guild_id, voicedata);

            let pcm = RawAdapter::new(session, 48000, 2);
            let _ = handler.play_input(pcm.into());
        }
        Err(err) => {
            respond_command(&ctx, interaction, &("Error: ".to_owned()+&err.to_string())).await;
            return;
        },
    }

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
        .content("Joined your voice channel. No module is currently playing".to_string())
    );
    interaction.create_response(ctx.http, response).await.unwrap();
}

pub fn register() -> CreateCommand {
    CreateCommand::new("join").description("Join a voice channel")
}