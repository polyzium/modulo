use std::ffi::{CStr, CString};

use libopenmpt_sys::{
    openmpt_module_get_current_order, openmpt_module_get_current_pattern, openmpt_module_get_current_row, openmpt_module_get_current_speed, openmpt_module_get_current_tempo2, openmpt_module_get_duration_seconds, openmpt_module_get_metadata, openmpt_module_get_num_channels, openmpt_module_get_num_instruments, openmpt_module_get_num_orders, openmpt_module_get_num_patterns, openmpt_module_get_num_samples, openmpt_module_get_num_subsongs, openmpt_module_get_position_seconds
};
use serenity::all::{CommandInteraction, Context};
use serenity::builder::CreateCommand;

use crate::botdata::BotDataKey;
use crate::misc::{followup_command, format_duration, respond_command};

const METADATA_KEYS: [&str; 11] = [
    "type",
    "type_long",
    "originaltype",
    "originaltype_long",
    "container",
    "container_long",
    "tracker",
    "artist",
    "title",
    "date",
    "warnings",
];

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
        // Title
        let title_key = CString::new("title").unwrap();
        let mut title: String = unsafe {CStr::from_ptr(openmpt_module_get_metadata(current_module.module.0, title_key.as_ptr()))}
            .to_str().unwrap()
            .to_string();
        if title.is_empty() {
            title = "[No title]".to_owned()
        }

        // Details
        let num_channels = unsafe {openmpt_module_get_num_channels(current_module.module.0)};
        let num_instruments = unsafe {openmpt_module_get_num_instruments(current_module.module.0)};
        let num_orders = unsafe {openmpt_module_get_num_orders(current_module.module.0)};
        let num_patterns = unsafe {openmpt_module_get_num_patterns(current_module.module.0)};
        let num_samples = unsafe {openmpt_module_get_num_samples(current_module.module.0)};
        let num_subsongs = unsafe {openmpt_module_get_num_subsongs(current_module.module.0)};

        let details = String::new() +
            "## Module details\n"+
            "Channels: "+&num_channels.to_string()+"\n"+
            "Instruments: "+&num_instruments.to_string()+"\n"+
            "Orders: "+&num_orders.to_string()+"\n"+
            "Patterns: "+&num_patterns.to_string()+"\n"+
            "Samples: "+&num_samples.to_string()+"\n"+
            "Subsongs: "+&num_subsongs.to_string();

        // Playback
        let position_sec = unsafe {openmpt_module_get_position_seconds(current_module.module.0)};
        let position = std::time::Duration::from_secs_f64(position_sec);
        let position_formatted = format_duration(position);

        let duration_sec = unsafe {openmpt_module_get_duration_seconds(current_module.module.0)};
        let duration = std::time::Duration::from_secs_f64(duration_sec);
        let duration_formatted = format_duration(duration);

        let order = unsafe {openmpt_module_get_current_order(current_module.module.0)};
        let pattern = unsafe {openmpt_module_get_current_pattern(current_module.module.0)};
        // let playing_channels = unsafe {openmpt_module_get_current_playing_channels(current_module.module.0)};
        let row = unsafe {openmpt_module_get_current_row(current_module.module.0)};
        let speed = unsafe {openmpt_module_get_current_speed(current_module.module.0)};
        let tempo = unsafe {openmpt_module_get_current_tempo2(current_module.module.0)};

        let playback = String::new() +
            "## Playback details\n"+
            &format!("Row {}, order {} (pattern {})", row, order, pattern)+"\n"+
            // "Playing_channels: "+&playing_channels.to_string()+"\n"+
            &format!("Speed/tempo: {}/{}", speed, tempo)+"\n"+
            &format!("Position/duration: {}/{}", position_formatted, duration_formatted);

        // Metadata
        let mut metadata = "## Metadata\n".to_string();
        for key in METADATA_KEYS {
            let raw_key = CString::new(key).unwrap();
            let value: String = unsafe {CStr::from_ptr(openmpt_module_get_metadata(current_module.module.0, raw_key.as_ptr()))}
                .to_str().unwrap()
                .to_string();

            if !value.is_empty() {
                metadata.push_str(&(key.to_string()+": "+&value+"\n"));
            }
        }

        drop(session_lock);
        let response = format!("# {title}\n{details}\n{playback}\n{metadata}");
        respond_command(&ctx, interaction, &response).await;
        return;
    } else {
        drop(session_lock);
        respond_command(&ctx, interaction, "No module is currently playing").await;
        return;
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("info").description("Print info about currently playing module")
}