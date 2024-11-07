use std::{ffi::{c_char, CStr}, os::raw::c_void};

use serenity::all::{CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage};

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