use std::ffi::{CStr, CString};

use libopenmpt_sys::openmpt_module_get_metadata;
use serenity::all::{CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::builder::CreateCommand;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let session = session_u.unwrap().clone();
    let session_lock = session.data.read().await;

    if let Some(current_module) = &session_lock.current_module {
        let mut song_message = "## Song message\n```".to_string();
        let message_key = CString::new("message").unwrap();
        let msg: String = unsafe {CStr::from_ptr(openmpt_module_get_metadata(current_module.module.0, message_key.as_ptr()))}
            .to_str().unwrap()
            .to_string()
            .replace("`", "\\`");
        song_message.push_str(&(msg.clone()+&"```"));

        drop(session_lock);

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
            .content(song_message)
        );
        let result = interaction.create_response(&ctx.http, response).await;
        if let Err(err) = result {
            match err {
                serenity::Error::Model(model_err) => {
                    if matches!(model_err, serenity::all::ModelError::MessageTooLong(_)) {
                        respond_command(&ctx, interaction, "We've reached Discord's character limit. Sorry.").await
                    } else {
                        respond_command(&ctx, interaction, &("Error: ".to_owned() + &model_err.to_string())).await
                    }
                },
                _ => log::warn!("Unable to send response: {err}")
            }
        }

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