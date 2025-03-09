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

mod events;
mod commands;
mod botdata;
mod session;
mod misc;
mod vote;

use std::env;
use botdata::{BotData, BotDataKey};
use events::Handler;
use serenity::prelude::*;

use songbird::SerenityInit;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    // tracing_subscriber::fmt()
    //     .compact()
    //     .finish();
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents =
        GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_PRESENCES
        | GatewayIntents::GUILDS
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    log::info!("Tracker bot initializing");
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .register_songbird()
        .await.expect("Err creating client");
    client.data.write().await.insert::<BotDataKey>(BotData::default());

    // Shut down on ctrl+C
    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        loop {
            sigint.recv().await;
            log::info!("Shutting down, goodbye");
            shard_manager.shutdown_all().await;
        }
    });

    if let Err(why) = client.start().await {
        log::error!("Client error: {why:?}");
    }
}