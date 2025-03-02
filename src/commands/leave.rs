
use serenity::all::{CommandInteraction, Context};
use serenity::builder::CreateCommand;

use crate::misc::{leave_vc, respond_command};

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let guild_id = interaction.guild_id.unwrap();

    if let Err(error) = leave_vc(&ctx, guild_id).await {
        respond_command(&ctx, interaction, &format!("{}", error)).await;
        return;
    }

    respond_command(&ctx, interaction, "Left the voice channel").await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("leave").description("Leave a voice channel")
}