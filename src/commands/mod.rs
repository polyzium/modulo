mod play;
mod join;
mod leave;
mod queue;
mod info;
mod jump;
mod message;
mod callvote;
mod pause;
mod resume;
mod stop;
mod interpolation;
mod amigaresampler;
mod readme;
mod autosubsong;

use std::sync::Arc;
use serenity::{all::{Command, CommandInteraction, Context, GuildId, Http}, Error};

pub async fn register_commands(http: &Arc<Http>) -> Result<Vec<Command>, Error> {
    Command::create_global_command(http, play::register()).await.unwrap();
    Command::create_global_command(http, join::register()).await.unwrap();
    Command::create_global_command(http, leave::register()).await.unwrap();
    Command::create_global_command(http, queue::register()).await.unwrap();
    Command::create_global_command(http, info::register()).await.unwrap();
    Command::create_global_command(http, jump::register()).await.unwrap();
    Command::create_global_command(http, message::register()).await.unwrap();
    Command::create_global_command(http, callvote::register()).await.unwrap();
    Command::create_global_command(http, pause::register()).await.unwrap();
    Command::create_global_command(http, resume::register()).await.unwrap();
    Command::create_global_command(http, stop::register()).await.unwrap();
    Command::create_global_command(http, interpolation::register()).await.unwrap();
    Command::create_global_command(http, amigaresampler::register()).await.unwrap();
    Command::create_global_command(http, readme::register()).await.unwrap();

    /*
        To anybody who comes across this line:
        this is purely for testing purposes as global commands
        take up to an hour to update, and thus are not instantaneous.
        Instead we make guild commands which update instantaneously.
    */
    let guild_id = GuildId::new(1302224187024216175);
    guild_id
        .set_commands(&http, vec![
            play::register(),
            join::register(),
            leave::register(),
            queue::register(),
            info::register(),
            jump::register(),
            message::register(),
            callvote::register(),
            pause::register(),
            resume::register(),
            stop::register(),
            interpolation::register(),
            amigaresampler::register(),
            readme::register(),
        ])
        .await
}

pub async fn handle_commands(ctx: Context, interaction: &CommandInteraction) {
    match interaction.data.name.as_str() {
        "play" => play::handle(ctx, interaction).await,
        "join" => join::handle(ctx, interaction).await,
        "leave" => leave::handle(ctx, interaction).await,
        "queue" => queue::handle(ctx, interaction).await,
        "info" => info::handle(ctx, interaction).await,
        "jump" => jump::handle(ctx, interaction).await,
        "message" => message::handle(ctx, interaction).await,
        "callvote" => callvote::handle(ctx, interaction).await,
        "pause" => pause::handle(ctx, interaction).await,
        "resume" => resume::handle(ctx, interaction).await,
        "stop" => stop::handle(ctx, interaction).await,
        "interpolation" => interpolation::handle(ctx, interaction).await,
        "amigaresampler" => amigaresampler::handle(ctx, interaction).await,
        "readme" => readme::handle(ctx, interaction).await,
        &_ => {},
    };
}