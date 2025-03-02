use std::ffi::{c_void, CStr, CString};
use std::ptr::{null, null_mut};

use libopenmpt_sys::{openmpt_module_create_from_memory2, openmpt_module_get_duration_seconds, openmpt_module_get_metadata};
use serenity::all::{ChannelId, ChannelType, CommandInteraction, Context, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage, ResolvedValue};
use serenity::builder::CreateCommand;

use crate::botdata::BotDataKey;
use crate::misc::{escape_markdown, followup_command, format_duration, openmpt_logger, respond_command};
use crate::session::{initiate_session, OpenMptModuleSafe, WrappedModule};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_handle_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    let mut deferred = false;
    if session_handle_u.is_none() {
        // respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        // return;

        let (guild_id, voice_channel_id) = {
            let guild_id = interaction.guild_id.unwrap();
            let guild = ctx.cache.guild(guild_id).unwrap();
            let voicestate_u = guild.voice_states.get(&interaction.member.clone().unwrap().user.id);
            let voice_channel_id: Option<ChannelId> = {
                if let Some(voicestate) = voicestate_u {
                    voicestate.channel_id
                } else {
                    None
                }
            };

            (guild_id, voice_channel_id)
        };

        let connect_to = match voice_channel_id {
            Some(channel) => channel,
            None => {
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                    .content("Join a voice channel first".to_string())
                );
                interaction.create_response(ctx.http, response).await.unwrap();
                return;
            },
        };

        interaction.defer(&ctx).await.unwrap();
        deferred = true;

        drop(data_lock);
        if let Err(err) = initiate_session(&ctx, guild_id, connect_to, interaction.channel_id).await {
            followup_command(&ctx, interaction, &err.to_string()).await;
            return;
        }
    }

    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());

    let url_u = interaction.data.options().iter()
        .find(|option| option.name == "url")
        .unwrap().clone()
        .value;
    let ResolvedValue::String(url) = url_u else { unreachable!() };

    // Defer the interaction because we're about to download the file
    if !deferred {
        interaction.defer(&ctx).await.unwrap();
    }
    let response_u = data_lock.get::<BotDataKey>().unwrap()
        .downloader_client.get(url).send().await;
    if let Err(err) = &response_u {
        followup_command(&ctx, interaction, &("HTTP request error: ".to_owned()+&err.to_string())).await;
        return;
    }
    let response = response_u.unwrap();
    if let Err(err) = response.error_for_status_ref() {
        followup_command(&ctx, interaction, &("Unable to fetch the module file: ".to_owned()+&err.to_string())).await;
        return;
    }

    let module_bytes = response.bytes()
        .await.unwrap();
    let module_file_hash = sha256::digest(&*module_bytes);

    let session = session_u.unwrap().clone();

    let module = OpenMptModuleSafe(unsafe {openmpt_module_create_from_memory2(
        module_bytes.as_ptr() as *const c_void,
        module_bytes.len(),
        Some(openmpt_logger),
        null_mut(),
        None,
        null_mut(),
        null_mut(),
        null_mut(),
        null(),
    )});
    if module.0.is_null() {
        let followup = CreateInteractionResponseFollowup::new()
        .content("Failed to initialize libopenmpt module.\nPlease make sure that the provided URL is a direct download link. For ModArchive modules, right click on \"Download\" and select \"Copy Link\" (on Firefox) or \"Copy link address\" (on Chrome).");
        interaction.create_followup(&ctx, followup).await.unwrap();

        return;
    }

    let r_url = reqwest::Url::parse(url).unwrap();
    let mut filename = String::new();

    // Detect ModArchive URL
    if r_url.host_str().unwrap() == "api.modarchive.org" {
        if let Some(fragment) = r_url.fragment() {
            // ModArchive URLs have their filename in the anchor/fragment area
            filename = fragment.to_string();
        }
    } else if let Some(segments) = r_url.path_segments() {
        if filename.is_empty() {
            // Take the last path segment and treat it as filename
            filename = segments.last().unwrap().to_string();
        }
    }

    let wrapped_module = WrappedModule {
        filehash: module_file_hash,
        module,
        filename,
    };

    let mut session_data_lock = session.data.write().await;
    let key = CString::new("title").unwrap();

    let loaded_module_title = unsafe {CStr::from_ptr(openmpt_module_get_metadata(wrapped_module.module.0, key.as_ptr()))}
        .to_str().unwrap();
    if let Some(playing_module) = &session_data_lock.current_module {
        if wrapped_module.filehash == playing_module.filehash {
            let followup = CreateInteractionResponseFollowup::new()
                .content("This module is already being played");
            drop(session_data_lock);
            interaction.create_followup(&ctx, followup).await.unwrap();
            return;
        }
    }

    for queued_module in session_data_lock.module_queue.iter() {
        if wrapped_module.filehash == queued_module.filehash {
            let followup = CreateInteractionResponseFollowup::new()
                .content("This module already exists in the queue");
            drop(session_data_lock);
            interaction.create_followup(&ctx, followup).await.unwrap();
            return;
        }
    }

    // Escape symbols that might conflict with Discord's Markdown syntax
    let mut loaded_module_title_escaped = escape_markdown(loaded_module_title);
    if loaded_module_title_escaped.is_empty() {
        loaded_module_title_escaped = wrapped_module.filename.clone()
    }

    let followup: CreateInteractionResponseFollowup;
    if session_data_lock.current_module.is_none() {
        session_data_lock.current_module = Some(wrapped_module);
        followup = CreateInteractionResponseFollowup::new()
            .content(&("Now playing: **".to_string()+&loaded_module_title_escaped+"**"));
    } else {
        let duration_sec = unsafe {openmpt_module_get_duration_seconds(wrapped_module.module.0)};
        let duration = std::time::Duration::from_secs_f64(duration_sec);
        let duration_formatted = format_duration(duration);
        session_data_lock.module_queue.push_back(wrapped_module);
        followup = CreateInteractionResponseFollowup::new()
            .content("Added **".to_string() + &loaded_module_title_escaped + "** (" + &duration_formatted + ") to the queue");
    }
    drop(session_data_lock);

    interaction.create_followup(&ctx, followup)
        .await.unwrap();
}

pub fn register() -> CreateCommand {
    CreateCommand::new("play").description("Play a module")
        .add_option(CreateCommandOption::new(serenity::all::CommandOptionType::String, "url", "Tracker module file URL").required(true))
}