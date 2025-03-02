use std::ffi::{CStr, CString};

use libopenmpt_sys::{openmpt_module_get_duration_seconds, openmpt_module_get_metadata, openmpt_module_get_position_seconds};
use serenity::all::{CommandInteraction, Context, CreateCommand};

use crate::{botdata::BotDataKey, misc::{escape_markdown, format_duration, respond_command}};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_handle_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_handle_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let session = session_handle_u.unwrap().clone();
    let session_data_lock = session.data.read().await;
    let queue = &session_data_lock
        .module_queue;
    let key = CString::new("title").unwrap();
    let mut current_content = String::new();
    let mut queue_content = String::from("Current song queue:\n");

    if let Some(current_module) = &session_data_lock.current_module {
        let mut title: String = unsafe {CStr::from_ptr(openmpt_module_get_metadata(current_module.module.0, key.as_ptr()))}
            .to_str().unwrap()
            .to_string();
        title = escape_markdown(&title);
        let mut paused = String::new();
        if title.is_empty() {
            title = current_module.filename.clone();
        }
        if session_data_lock.paused {
            paused = " (paused)".to_string();
        }

        let position_sec = unsafe {openmpt_module_get_position_seconds(current_module.module.0)};
        let position = std::time::Duration::from_secs_f64(position_sec);
        let position_formatted = format_duration(position);

        let duration_sec = unsafe {openmpt_module_get_duration_seconds(current_module.module.0)};
        let duration = std::time::Duration::from_secs_f64(duration_sec);
        let duration_formatted = format_duration(duration);

        current_content = "Currently playing: **".to_string()+&title+"** " + &format!("({}/{})", position_formatted, duration_formatted) + &paused + "\n";
    }

    if queue.is_empty() {
        queue_content = "The queue is empty. Use /play to pick a song.".to_string();
    } else {
        for (i, queued_module) in queue.iter().enumerate() {
            let duration_sec = unsafe {openmpt_module_get_duration_seconds(queued_module.module.0)};
            let duration = std::time::Duration::from_secs_f64(duration_sec);
            let duration_formatted = format_duration(duration);

            let mut title: String = unsafe {CStr::from_ptr(openmpt_module_get_metadata(queued_module.module.0, key.as_ptr()))}
                .to_str().unwrap()
                .to_string();
            title = escape_markdown(&title);
            if title.is_empty() {
                title = queued_module.filename.clone();
            }

            queue_content.push_str(&((i+1).to_string()+": **"+&title+"** ("+&duration_formatted+")\n"));
        }
    }
    drop(session_data_lock);

    respond_command(&ctx, interaction, &(current_content.clone()+&queue_content)).await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("queue").description("View the current song queue")
}