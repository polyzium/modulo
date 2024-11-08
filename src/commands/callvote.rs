use std::collections::HashMap;
use std::time::Duration;

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

    let session = session_u.unwrap().clone();
    let session_lock = session.data.read().await;

    if session_lock.current_vote.is_some() {
        drop(session_lock);
        respond_command(&ctx, interaction, "There is already a vote in progress").await;
        return;
    }
    // We're gonna fetch a bunch of user data so unlock the session
    // to prevent audio glitches. We'll lock it later
    drop(session_lock);

    let vote_kind_string = {
        if let ResolvedValue::SubCommand(_) = interaction.data.options()[0].value {
            interaction.data.options()[0].name
        } else { unreachable!() }
    };
    let vote_kind = match vote_kind_string {
        "skip" => VoteKind::Skip,
        &_ => unreachable!()
    };
    let bot_id = ctx.http.get_current_user().await.unwrap().id;
    let voice_channel_id = ctx.cache.guild(interaction.guild_id.unwrap()).unwrap()
        .voice_states.get(&bot_id).unwrap()
        .channel_id.unwrap();
    // Below we subtract by 1 because the bot also counts as a member.
    // the bot doesn't need to cast a vote.
    let amount_users_in_vc = ctx.http.get_channel(voice_channel_id).await.unwrap()
        .guild().unwrap()
        .members(&ctx).unwrap()
        .len()-1;

    let (death_tx, mut death_rx) = channel::<bool>(4);
    let mut vote = Vote {
        kind: vote_kind,
        votes_cast: HashMap::new(),
        votes_needed: amount_users_in_vc,
        timer_death_handle: death_tx.clone(),
        text_channel_id: interaction.channel_id,
    };
    // End vote right away if only one vote is required
    if vote.votes_needed == 1 {
        session.data.write().await
            .current_vote = Some(vote);
        end_vote(ctx.clone(), &session).await;
        respond_command(&ctx, interaction, "Only one user inside the voice channel; assuming vote passed.").await;
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
}