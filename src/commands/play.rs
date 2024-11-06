use std::ffi::c_void;
use std::ptr::{null, null_mut};

use libopenmpt_sys::openmpt_module_create_from_memory2;
use serenity::all::{CommandInteraction, Context, CreateCommandOption, CreateInteractionResponseFollowup, ResolvedValue};
use serenity::builder::CreateCommand;

use crate::botdata::BotDataKey;
use crate::misc::{openmpt_logger, respond_command};
use crate::session::OpenMptModuleSafe;

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_data_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_data_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let url_u = interaction.data.options().iter()
        .find(|option| option.name == "url")
        .unwrap().clone()
        .value;
    let ResolvedValue::String(url) = url_u else { unreachable!() };
    // Defer an interaction because we're about to download the file.
    interaction.defer(&ctx).await.unwrap();
    let response = data_lock.get::<BotDataKey>().unwrap()
        .downloader_client.get(url).send().await.unwrap();
    if let Err(err) = response.error_for_status_ref() {
        respond_command(&ctx, interaction, &("Error: ".to_owned()+&err.to_string())).await;
    }

    let module_bytes = response.bytes()
        .await.unwrap();

    let session_data = session_data_u.unwrap().clone();

    let raw_openmpt_module = unsafe {openmpt_module_create_from_memory2(
        module_bytes.as_ptr() as *const c_void,
        module_bytes.len(),
        Some(openmpt_logger),
        null_mut(),
        None,
        null_mut(),
        null_mut(),
        null_mut(),
        null(),
    )};
    if raw_openmpt_module.is_null() {
        let followup = CreateInteractionResponseFollowup::new()
        .content("Failed to initialize libopenmpt module");
        interaction.create_followup(&ctx, followup).await.unwrap();

        return;
    }

    session_data.write().unwrap()
        .module = Some(OpenMptModuleSafe(raw_openmpt_module));

    let followup = CreateInteractionResponseFollowup::new()
        .content("Playing song");
    interaction.create_followup(&ctx, followup).await.unwrap();
}

pub fn register() -> CreateCommand {
    CreateCommand::new("play").description("Play a module")
        .add_option(CreateCommandOption::new(serenity::all::CommandOptionType::String, "url", "Tracker module file URL").required(true))
}