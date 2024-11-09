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

    if session_lock.current_module.is_some() {
        session_lock.current_module = None;
        session_lock.paused = false;
    } else {
        drop(session_lock);
        respond_command(&ctx, interaction, "No module is currently playing").await;
        return;
    }

    drop(session_lock);
    respond_command(&ctx, interaction, "Playback stopped").await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("stop").description("Stop playback")
}