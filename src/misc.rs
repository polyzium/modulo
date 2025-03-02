use std::{ffi::{c_char, CStr}, os::raw::c_void, time::Duration};

use serenity::all::{CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage, GuildId};
use anyhow::{anyhow, Result};

use crate::botdata::BotDataKey;

pub async fn respond_command(ctx: &Context, interaction: &CommandInteraction, text: &str) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
        .content(text.to_string())
    );
    interaction.create_response(&ctx.http, response).await.unwrap();
}

pub async fn followup_command(ctx: &Context, interaction: &CommandInteraction, text: &str) {
    let response =
        CreateInteractionResponseFollowup::new()
        .content(text.to_string());
    interaction.create_followup(&ctx.http, response).await.unwrap();
}

pub unsafe extern "C" fn openmpt_logger(message: *const c_char, user: *mut c_void) {
    let msg = CStr::from_ptr(message).to_str().unwrap();
    log::info!("{}", msg);
}

// pub unsafe extern "C" fn openmpt_logger_err(message: *const c_char, user: *mut c_void) {
//     let msg = CStr::from_ptr(message).to_str().unwrap();
//     log::error!("{}", msg);
// }

// pub fn openmpt_ctls(kv: HashMap<String, String>) -> *const c_char {

// }

// pub fn openmpt_ctls_ordered(kv: &[(String, String)]) -> *const c_char {
    
// }

// pub async fn check_dj_role(ctx: &Context, guild_id: GuildId) {
//     ctx.cache.guild(guild_id).unwrap()
//         .role_by_name("DJ");
// }

pub fn escape_markdown(string: &str) -> String {
    let mut new_string: String = String::new();
    let characters_to_escape = "*_~[]()<>-#`\\~";
    for c in characters_to_escape.chars() {
        new_string = string.replace(c, &("\\".to_string()+&c.to_string()));
    };

    new_string
}

pub fn format_duration(duration: Duration) -> String {
    format!("{}:{:0>2}", duration.as_secs()/60, duration.as_secs()%60)
}

pub async fn leave_vc(ctx: &Context, guild_id: GuildId) -> Result<()> {
    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Err(err) = manager.remove(guild_id).await {
        match err {
            songbird::error::JoinError::NoCall => return Err(anyhow!("The bot is not in a voice channel")),
            _ => return Err(anyhow::Error::new(err).context("Error leaving the voice channel"))
        }
    }

    let mut lock = ctx.data.write().await;
    let botdata = lock.get_mut::<BotDataKey>().unwrap();

    let session = botdata.sessions.get(&guild_id).cloned().unwrap();
    let session_lock = session.data.write().await;
    botdata.sessions.remove(&guild_id);
    drop(lock);

    let handle = session_lock.notification_handle.clone();
    handle.send(crate::session::VoiceSessionNotificationMessage::Leave).await.unwrap();
    if let Some(vote) = &session_lock.current_vote {
        vote.timer_death_handle.send(false).await.unwrap();
    }
    drop(session_lock);

    Ok(())
}