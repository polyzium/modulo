use serenity::all::{CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::builder::CreateCommand;

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
        .content("pong".to_string())
    );
    interaction.create_response(ctx.http, response).await.unwrap();
}

pub fn register() -> CreateCommand {
    CreateCommand::new("ping").description("A ping command")
}