use std::ffi::{CStr, CString};

use libopenmpt_sys::openmpt_module_get_metadata;
use serenity::all::{CommandInteraction, Context};
use serenity::builder::CreateCommand;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_data_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_data_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let session_data = session_data_u.unwrap().clone();
    let session_lock = session_data.read().await;

    if let Some(current_module) = &session_lock.current_module {
        let mut song_message = "## Song message\n```".to_string();
        let message_key = CString::new("message").unwrap();
        let msg: String = unsafe {CStr::from_ptr(openmpt_module_get_metadata(current_module.module.0, message_key.as_ptr()))}
            .to_str().unwrap()
            .to_string();
        song_message.push_str(&(msg.clone()+&"```"));

        drop(session_lock);
        respond_command(&ctx, interaction, &song_message).await;
        return;
    } else {
        drop(session_lock);
        respond_command(&ctx, interaction, "No module is currently playing").await;
        return;
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("message").description("Print currently playing module's song message")
}