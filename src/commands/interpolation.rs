use libopenmpt_sys::{openmpt_module_set_render_param, OPENMPT_MODULE_RENDER_INTERPOLATIONFILTER_LENGTH};
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

    let interpolation_string = {
        if let ResolvedValue::SubCommand(_) = interaction.data.options()[0].value {
            interaction.data.options()[0].name
        } else { unreachable!() }
    };
    let interpolation = match interpolation_string {
        "default" => Interpolation::Default,
        "none" => Interpolation::None,
        "linear" => Interpolation::Linear,
        "cubic" => Interpolation::Cubic,
        "sinc8" => Interpolation::Sinc8,
        &_ => unreachable!()
    };

    let session = session_u.cloned().unwrap();
    let mut session_lock = session.data.write().await;
    session_lock.interpolation = interpolation;
    if let Some(current_module) = &mut session_lock.current_module {
        unsafe {openmpt_module_set_render_param(
            current_module.module.0,
            OPENMPT_MODULE_RENDER_INTERPOLATIONFILTER_LENGTH as std::os::raw::c_int,
            session_lock.interpolation.to_openmpt_value())
        };
    }
    drop(session_lock);

    respond_command(&ctx, interaction, &("Interpolation changed to **".to_owned()+&interpolation_string+"**")).await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("interpolation").description("Change interpolation for this session")
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "default", "Internal default of libopenmpt"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "none", "No interpolation"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "linear", "Linear interpolation"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "cubic", "Cubic interpolation"))
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "sinc8", "Windowed sinc with 8 taps interpolation"))
}