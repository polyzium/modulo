use serenity::all::ComponentInteractionDataKind;
use serenity::all::Interaction;
use serenity::all::Ready;
use serenity::prelude::*;
use serenity::async_trait;
use serenity::model::channel::Message;

use crate::commands;
use crate::vote;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                log::error!("Error sending message: {why:?}");
            }
        }
    }

    async fn ready(&self, ctx: Context, botdata: Ready) {
        log::info!("Logged in as {}", botdata.user.name);
        if let Err(err) = commands::register_commands(&ctx.http).await {
            log::error!("Unable to register commands: {}", err.to_string())
        };
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(ref command) => commands::handle_commands(ctx, command).await,
            Interaction::Component(ref component_interaction) => {
                if let ComponentInteractionDataKind::Button = component_interaction.data.kind {
                    if component_interaction.data.custom_id.starts_with("vote") {
                        vote::handle_voting(ctx, component_interaction).await;
                    }
                }
            },
            _ => (),
        }
    }

    // async fn voice_state_update
}