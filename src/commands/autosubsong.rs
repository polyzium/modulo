use serenity::all::{CommandInteraction, Context, CreateCommand, CreateCommandOption, ResolvedValue};

use crate::{botdata::BotDataKey, misc::respond_command};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let enabled = {
        if let ResolvedValue::Boolean(value) = interaction.data.options()[0].value {
            value
        } else { unreachable!() }
    };

    let session = session_u.cloned().unwrap();
    let mut session_lock = session.data.write().await;
    session_lock.autosubsong_enabled = enabled;

    drop(session_lock);

    let text = if enabled {
        "The bot will now play all subsongs in a module"
    } else {
        "The bot will no longer play all subsongs in a module"
    };
    respond_command(&ctx, interaction, text).await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("autosubsong").description("Whether to automatically play all subsongs in a module")
        .add_option(
            CreateCommandOption::new(serenity::all::CommandOptionType::Boolean, "enabled", "Should the bot play all subsongs?")
                .required(true)
    )
}