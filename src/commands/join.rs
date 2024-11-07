use serenity::all::{ChannelId, ChannelType, CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::builder::CreateCommand;
use songbird::input::RawAdapter;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;
use crate::session::{initiate_session, VoiceSession};

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

    if let Err(err) = initiate_session(&ctx, guild_id, connect_to, interaction.channel_id).await {
        respond_command(&ctx, interaction, &err.to_string()).await;
        return;
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