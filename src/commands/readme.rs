use serenity::all::{CommandInteraction, Context, CreateCommand};

use crate::misc::respond_command;

// I've seen a lot of people add Modulo without understanding what the hell it's for,
// expecting it to play YouTube videos or MP3s.
// So I made this. I hope this clears up the confusion.

const README_TEXT: &'static str = "# Hi!\nThis is Modulo, a Discord music bot for playing back tracker music files (aka MOD music files or just tracker modules).\nIf you added this bot expecting it to play YouTube videos or MP3s, **this is not the bot for you**. This bot is mainly intended to be used in tracker or demoscene centric communities.\nOtherwise, enjoy the music!\n\nPlease note that the bot is in early stages of development. See the Help section below for more details.\n# Getting started\nStart by invoking the /play command, to which you should provide a URL of the module file you want to play. The bot will join whatever voice channel you are in (otherwise it will fail). Invoking /play again during a playing module will put the provided module into the queue.\n# Help\nPlease make an issue on the [GitHub issue tracker](https://github.com/polyzium/modulo/issues) or contact `polyzium` on Discord for any serious issues that may arise.";

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    respond_command(&ctx, interaction, &README_TEXT).await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("readme").description("If you're new, please run this!")
}