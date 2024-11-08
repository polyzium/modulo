
use serenity::all::{CommandInteraction, Context};
use serenity::builder::CreateCommand;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let guild_id = interaction.guild_id.unwrap();

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Err(err) = manager.remove(guild_id).await {
        match err {
            songbird::error::JoinError::NoCall => respond_command(&ctx, interaction, "The bot is not in a voice channel").await,
            _ => respond_command(&ctx, interaction, &("Error: ".to_owned()+&err.to_string())).await,
        }
        return
    }

    let mut lock = ctx.data.write().await;
    let botdata = lock.get_mut::<BotDataKey>().unwrap();
    let guild_id = interaction.guild_id.unwrap();

    let session = botdata.sessions.get(&guild_id).unwrap();
    let session_lock = session.data.write().await;

    let handle = session_lock.notification_handle.clone();
    handle.send(crate::session::VoiceSessionNotificationMessage::Leave).await.unwrap();
    drop(session_lock);
    botdata.sessions.remove(&guild_id);

    respond_command(&ctx, interaction, "Left the voice channel").await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("leave").description("Leave a voice channel")
}