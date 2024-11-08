use serenity::all::{CacheHttp, ChannelId, ChannelType, CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::builder::CreateCommand;
use songbird::input::RawAdapter;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;
use crate::session::{initiate_session, VoiceSession};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let (guild_id, voice_channel_id) = {
        let guild_id = interaction.guild_id.unwrap();
        let guild = ctx.cache.guild(guild_id).unwrap();
        let voicestate_u = guild.voice_states.get(&interaction.member.clone().unwrap().user.id);
        let voice_channel_id: Option<ChannelId> = {
            if let Some(voicestate) = voicestate_u {
                voicestate.channel_id
            } else {
                None
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