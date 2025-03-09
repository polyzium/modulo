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

use std::ffi::CString;

use libopenmpt_sys::{openmpt_module_ctl_set_boolean, openmpt_module_ctl_set_text, openmpt_module_set_render_param, OPENMPT_MODULE_RENDER_INTERPOLATIONFILTER_LENGTH};
use serenity::all::{CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption, ResolvedValue};

use crate::{botdata::BotDataKey, misc::respond_command, session::Interpolation};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let mode_string = {
        if let ResolvedValue::SubCommand(_) = interaction.data.options()[0].value {
            interaction.data.options()[0].name
        } else { unreachable!() }
    };

    let session = session_u.cloned().unwrap();
    let mut session_lock = session.data.write().await;
    let enabled =  mode_string != "none";
    session_lock.amiga_enabled = enabled;
    if let Some(current_module) = &session_lock.current_module {
        let ctl = CString::new("render.resampler.emulate_amiga").unwrap();
        unsafe { openmpt_module_ctl_set_boolean(current_module.module.0, ctl.as_ptr(), enabled as i32) };
    };

    if !enabled {
        drop(session_lock);
        respond_command(&ctx, interaction, "Amiga resampler disabled").await;
        return;
    };

    session_lock.amiga_mode = mode_string.to_owned();
    if let Some(current_module) = &session_lock.current_module {
        let ctl = CString::new("render.resampler.emulate_amiga_type").unwrap();
        let value = CString::new(mode_string).unwrap();
        unsafe { openmpt_module_ctl_set_text(current_module.module.0, ctl.as_ptr(), value.as_ptr()) };
    };
    drop(session_lock);

    respond_command(&ctx, interaction, &("Amiga resampler changed to **".to_owned()+&mode_string+"**")).await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("amigaresampler").description("Change Amiga resampler mode for this session")
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "none", "Disable Amiga resampling"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "auto", "Internal default of libopenmpt"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "a500", "Amiga 500 filtering"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "a1200", "Amiga 1200 filtering"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "unfiltered", "BLEP synthesis without model-specific filters"))
}