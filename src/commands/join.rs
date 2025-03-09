/*
 * This file is part of Modulo.
 *
 * Copyright (C) 2024-present Polyzium
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

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