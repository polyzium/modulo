use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::time::Duration;

use libopenmpt_sys::{openmpt_module_get_duration_seconds, openmpt_module_get_metadata, openmpt_module_get_position_seconds};
use serenity::all::{ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateButton, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, Mentionable, ResolvedValue};
use serenity::builder::CreateCommand;
use tokio::spawn;
use tokio::sync::mpsc::channel;
use tokio::time::sleep;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;
use crate::vote::{end_vote, Vote, VoteKind};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }

    let ResolvedValue::SubCommand(ref vote_options) = interaction.data.options()[0].value else { unreachable!() };
    let vote_kind_string = {
        if let ResolvedValue::SubCommand(_) = interaction.data.options()[0].value {
            interaction.data.options()[0].name
        } else { unreachable!() }
    };

    let session = session_u.unwrap().clone();
    let session_lock = session.data.read().await;

    if session_lock.current_vote.is_some() && vote_kind_string != "cancel" {
        drop(session_lock);
        respond_command(&ctx, interaction, "There is already a vote in progress").await;
        return;
    }
    // We're gonna fetch a bunch of user data so unlock the session
    // to prevent audio glitches. We'll lock it later
    drop(session_lock);

    if vote_kind_string == "cancel" {
        let mut session_lock = session.data.write().await;
        if session_lock.current_vote.is_none() {
            drop(session_lock);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("No vote in progress")
                    .ephemeral(true)
            );
            interaction.create_response(&ctx, response).await.unwrap();
            return;
        }
        let vote = session_lock.current_vote.as_ref()
            .unwrap();
        if vote.caller != interaction.member.clone().unwrap().user.id {
            drop(session_lock);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("You are not allowed to cancel the current vote")
                    .ephemeral(true)
            );
            interaction.create_response(&ctx, response).await.unwrap();
            return;
        }
        let votecaller_user = ctx.cache.user(vote.caller).unwrap().clone();
        vote.timer_death_handle.send(false).await.unwrap();
        session_lock.current_vote = None;
        drop(session_lock);

        respond_command(&ctx, interaction, &("# Vote cancelled by ".to_owned()+&votecaller_user.mention().to_string())).await;
        return;
    }

    match vote_kind_string {
        "skip" | "delsong" => {
            let session_lock = session.data.read().await;
            if session_lock.current_module.is_none() {
                drop(session_lock);
                respond_command(&ctx, interaction, "No module is playing, can't call a vote of this kind").await;
                return;
            }
            let current_module = session_lock.current_module.as_ref().unwrap();
            let position = unsafe { openmpt_module_get_position_seconds(current_module.module.0) };
            let duration = unsafe { openmpt_module_get_duration_seconds(current_module.module.0) };
            if duration-position < 30.0 {
                drop(session_lock);
                respond_command(&ctx, interaction, "Less than 30 seconds of this module remaining, can't call a vote of this kind").await;
                return;
            }
        },
        &_ => {}
    }
    let vote_kind = match vote_kind_string {
        "skip" => VoteKind::Skip,
        "delsong" => {
            let index_option = vote_options.iter()
                .find(|option| option.name == "which")
                .unwrap()
                .value.clone();
            let ResolvedValue::Integer(mut index) = index_option else { unreachable!() };
            index -= 1;
            if index.is_negative() {
                respond_command(&ctx, interaction, "Value cannot be zero or negative").await;
                return;
            }

            let session_lock = session.data.read().await;

            if index as usize >= session_lock.module_queue.len() {
                drop(session_lock);
                respond_command(&ctx, interaction, "Out of range").await;
                return;
            }
            let module_to_remove = &session_lock.module_queue[index as usize];

            let title_key = CString::new("title").unwrap();
            let mut title: String = unsafe {CStr::from_ptr(openmpt_module_get_metadata(module_to_remove.module.0, title_key.as_ptr()))}
                .to_str().unwrap()
                .to_string();
            if title.is_empty() {
                title = "[No title]".to_owned()
            }

            VoteKind::RemoveSongFromQueue(index as usize, title)
        },
        &_ => unreachable!()
    };
    let bot_id = ctx.http.get_current_user().await.unwrap().id;
    let voice_channel_id = ctx.cache.guild(interaction.guild_id.unwrap()).unwrap()
        .voice_states.get(&bot_id).unwrap()
        .channel_id.unwrap();
    // Below we subtract by 1 because the bot also counts as a member.
    // the bot doesn't need to cast a vote.
    let members = ctx.cache.guild(interaction.guild_id.unwrap()).unwrap()
        .channels.get(&voice_channel_id).unwrap()
        .members(&ctx).unwrap();
    let votes_needed = members
        .iter().filter(|member|
            !interaction.guild_id.unwrap()
                .to_guild_cached(&ctx).unwrap()
                .voice_states.get(&member.user.id).unwrap()
                .self_deaf
        )
        .count().saturating_sub(1);

    let (death_tx, mut death_rx) = channel::<bool>(4);
    let mut vote = Vote {
        caller: interaction.member.clone().unwrap().user.id,
        kind: vote_kind,
        votes_cast: HashMap::new(),
        votes_needed,
        timer_death_handle: death_tx.clone(),
        text_channel_id: interaction.channel_id,
    };
    // End vote right away if only one vote is required
    if vote.votes_needed <= 1 {
        session.data.write().await
            .current_vote = Some(vote);
        end_vote(ctx.clone(), &session).await;
        respond_command(&ctx, interaction, "Only one or no votes required; assuming vote passed.").await;
        return;
    }

    // Start a timer thread
    let ctx2 = ctx.clone();
    let session2 = session.clone();
    let mut seconds_remaining = 30;
    let text_channel_id = vote.text_channel_id;
    spawn(async move {
        loop {
            sleep(Duration::from_secs(1)).await;
            let death = death_rx.try_recv();
            seconds_remaining -= 1;
            if let Ok(died) = death {
                if !died {
                    return;
                }
            }
            if seconds_remaining == 15 || seconds_remaining == 5 {
                text_channel_id.send_message(&ctx2, CreateMessage::new().content(
                    seconds_remaining.to_string()+" seconds til the vote ends"
                )).await.unwrap();
            }
            if seconds_remaining == 0 {
                end_vote(ctx2, &session2).await;
                return;
            }
        }
    });

    // The vote caller presumably would automatically cast as "yes"
    let votecaller_user = interaction.member.clone().unwrap().user;
    vote.votes_cast.insert(votecaller_user.id, true);

    let vote_kind = (&vote).kind.clone();
    let mut session_lock = session.data.write().await;
    session_lock.current_vote = Some(vote);
    drop(session_lock);

    let response = CreateInteractionResponseMessage::new()
        .content(
            "# VOTE NOW\n".to_string() +
            &votecaller_user
                .mention().to_string() +
            " has started a vote to **" + &vote_kind.to_string() + "**\n" +
            &"*You have 30 seconds to cast a vote*"
        )
        .button(
            CreateButton::new("vote1").style(ButtonStyle::Success)
                .label("YES")
        )
        .button(
            CreateButton::new("vote0").style(ButtonStyle::Danger)
                .label("NO")
        );

    interaction.create_response(&ctx.http, CreateInteractionResponse::Message(response)).await.unwrap();
}

pub fn register() -> CreateCommand {
    CreateCommand::new("callvote").description("Call a vote")
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "skip", "Skip currently playing song"))
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "delsong", "Remove song from a queue")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "which", "Number/index of a song in the queue to be removed")
                    .required(true)
                )
        )
        .add_option(CreateCommandOption::new(CommandOptionType::SubCommand, "cancel", "Cancel currently ongoing vote"))
}