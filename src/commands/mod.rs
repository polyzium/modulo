mod ping;
mod play;
mod test;
mod join;
mod leave;

use std::sync::Arc;
use serenity::{all::{Command, CommandInteraction, Context, GuildId, Http}, Error};

pub async fn register_commands(http: &Arc<Http>) -> Result<Vec<Command>, Error> {
    // // Command::create_global_command(http, ping::register()).await.unwrap();
    // // Command::create_global_command(http, play::register()).await.unwrap();
    // // Command::create_global_command(http, test::register()).await.unwrap();
    // // Command::create_global_command(http, join::register()).await.unwrap();
    // let commands = Command::get_global_commands(&http).await.unwrap();
    // for cmd  in commands {
    //     Command::delete_global_command(&http, cmd.id).await.unwrap();
    // }

    // let guild_id = GuildId::new(1302224187024216175);
    let guild_id = GuildId::new(683394719770083424);

    // // let commands = guild_id
    // let command_ids = guild_id.get_commands(&http).await.unwrap()
    //     .iter_mut().map(|command| command.id)
    //     .collect::<Vec<CommandId>>();
    // for cmd_id  in command_ids {
    //     guild_id.delete_command(&http, cmd_id).await.unwrap();
    // }
    guild_id
        .set_commands(&http, vec![
            ping::register(),
            play::register(),
            test::register(),
            join::register(),
            leave::register(),
        ])
        .await
}

pub async fn handle_commands(ctx: Context, interaction: &CommandInteraction) {
    match interaction.data.name.as_str() {
        "ping" => ping::handle(ctx, interaction).await,
        "play" => play::handle(ctx, interaction).await,
        "test" => test::handle(ctx, interaction).await,
        "join" => join::handle(ctx, interaction).await,
        "leave" => leave::handle(ctx, interaction).await,
        &_ => {},
    };
}