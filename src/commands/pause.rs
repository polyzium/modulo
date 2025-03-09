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

use serenity::all::{CommandInteraction, Context, CreateCommand, CreateCommandOption};

use crate::{botdata::BotDataKey, misc::respond_command};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let session = session_u.cloned().unwrap();
    let mut session_lock = session.data.write().await;
    if session_lock.paused {
        drop(session_lock);
        respond_command(&ctx, interaction, "Already paused").await;
        return;
    }

    if session_lock.current_module.is_some() {
        session_lock.paused = true;
    } else {
        drop(session_lock);
        respond_command(&ctx, interaction, "No module is currently playing").await;
        return;
    }

    drop(session_lock);
    respond_command(&ctx, interaction, "Playback paused").await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("pause").description("Pause playback")
}