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