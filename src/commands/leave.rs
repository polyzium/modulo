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

use serenity::all::{CommandInteraction, Context};
use serenity::builder::CreateCommand;

use crate::misc::{leave_vc, respond_command};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let guild_id = interaction.guild_id.unwrap();

    if let Err(error) = leave_vc(&ctx, guild_id).await {
        respond_command(&ctx, interaction, &format!("{}", error)).await;
        return;
    }

    respond_command(&ctx, interaction, "Left the voice channel").await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("leave").description("Leave a voice channel")
}