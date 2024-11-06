use std::{collections::HashMap, ffi::{c_char, CStr}, os::raw::c_void, sync::Arc};

use serenity::all::{CommandData, CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage, Http, ResolvedOption};

pub async fn respond_command(ctx: &Context, interaction: &CommandInteraction, text: &str) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
        .content(text.to_string())
    );
    interaction.create_response(&ctx.http, response).await.unwrap();
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