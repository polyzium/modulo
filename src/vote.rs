/*
 * This file is part of Modulo.
 *
 * Copyright (C) 2024-present Polyzium
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::{collections::HashMap, sync::Arc};

use libopenmpt_sys::openmpt_module_get_metadata;
use serenity::all::{ChannelId, ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, GuildId, Mentionable, UserId};
use tokio::sync::{mpsc::Sender, RwLock};

use crate::{botdata::BotDataKey, session::{VoiceSession, VoiceSessionData, VoiceSessionHandle, VoiceSessionNotificationMessage}};

#[derive(Debug, Clone)]
pub enum VoteKind {
    Skip,
    RemoveSongFromQueue(usize, String)
}

pub enum VoteStatus {
    Passed,
    FailedNotEnough,
    FailedMostVotedNo
}

impl VoteKind {
    pub fn action_end_vote(&self) -> String {
        match self {
            VoteKind::Skip => "Skipping current song".to_owned(),
            VoteKind::RemoveSongFromQueue(_, title) => format!("Removed {} from the queue", title),
        }
    }
}

impl ToString for VoteKind {
    fn to_string(&self) -> String {
        match self {
            VoteKind::Skip => "skip current song".to_owned(),
            VoteKind::RemoveSongFromQueue(_, title) => format!("remove {} from the queue", title),
        }
    }
}

#[derive(Clone)]
pub struct Vote {
    pub caller: UserId,
    pub text_channel_id: ChannelId,
    pub kind: VoteKind,
    pub votes_needed: usize,
    pub votes_cast: HashMap<UserId, bool>,
    /// Because we run the timer in a separate thread, we need to tell it to stop
    /// if the vote has ended. That's what timer_handle is for.
    pub(crate) timer_death_handle: Sender<bool>
}

pub async fn handle_voting(ctx: Context, interaction: &ComponentInteraction) {
    let data_lock = ctx.data.read().await;
    let session_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_u.is_none() {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
            .content("The bot must be in a voice channel")
            .ephemeral(true)
        );
        interaction.create_response(&ctx.http, response).await.unwrap();
        return;
    }

    let session = session_u.unwrap().clone();
    let session_data = session.data.clone();
    drop(data_lock);

    let vote_choice = interaction.data.custom_id == "vote1";
    let member = interaction.member.clone().unwrap();
    let vote_choice_string = member
        .user.mention().to_string() + " voted " + { if vote_choice {"YES"} else {"NO"} };

    let mut session_lock = session_data.write().await;
    if let Some(vote) = &mut session_lock.current_vote {
        if vote.votes_cast.contains_key(&member.user.id) {
            drop(session_lock);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("You have already voted")
                    .ephemeral(true)
            );
            interaction.create_response(&ctx, response).await.unwrap();
            return;
        }

        vote.votes_cast.insert(member.user.id, vote_choice);
        if vote.votes_cast.keys().len() == vote.votes_needed {
            drop(session_lock);
            end_vote(ctx.clone(), &session).await;

            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(vote_choice_string)
            );
            interaction.create_response(&ctx, response).await.unwrap();

            return;
        }
    } else {
        drop(session_lock);
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("No vote in progress")
                .ephemeral(true)
        );
        interaction.create_response(&ctx, response).await.unwrap();
        return;
    }
    drop(session_lock);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(vote_choice_string)
    );
    interaction.create_response(&ctx, response).await.unwrap();
}

pub async fn end_vote(ctx: Context, session: &VoiceSessionHandle) {
    let session_lock = session.data.read().await;
    if session.control_tx.is_closed() {
        log::warn!("Session has been closed before the vote could end, ignoring");
        return;
    }
    if let Some(vote) = session_lock.current_vote.clone() {
        drop(session_lock);
        let death_handle = vote.timer_death_handle.clone();
        let yes_amount = vote.votes_cast.values()
            .filter(|vote| **vote)
            .count();
        let mut vote_override = false;
        if vote.votes_needed <= 1 {
            vote_override = true;
        }
        let no_amount = vote.votes_cast.values()
            .filter(|vote| !**vote)
            .count();

        if vote.votes_cast.keys().len() <= vote.votes_needed/2 && !vote_override {
            vote.text_channel_id.send_message(&ctx, CreateMessage::new().content(
                "# :x: VOTE FAILED\nNot enough users voted."
            )).await.unwrap();
        } else if no_amount >= yes_amount && !vote_override {
            vote.text_channel_id.send_message(&ctx, CreateMessage::new().content(
                "# :x: VOTE FAILED\nThe majority of users voted NO."
            )).await.unwrap();
        } else if yes_amount > no_amount || vote_override {
            vote.text_channel_id.send_message(&ctx, CreateMessage::new().content(
                "# :white_check_mark: VOTE PASSED\n".to_string()+&vote.kind.action_end_vote()
            )).await.unwrap();


            match vote.kind {
                VoteKind::Skip => {
                    session.control_tx.send(crate::session::VoiceSessionControlMessage::PlayNextInQueue).await.unwrap();
                },
                VoteKind::RemoveSongFromQueue(index, _) => {
                    let mut session_lock = session.data.write().await;
                    session_lock.module_queue.remove(index);
                    drop(session_lock);
                },
            }
        };

        session.data.write().await
            .current_vote = None;
        death_handle.send(false).await.unwrap();
    } else {
        log::warn!("Attempted to end an empty vote, ignoring.")
    }
}