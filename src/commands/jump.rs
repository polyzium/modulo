use libopenmpt_sys::{openmpt_module_get_current_order, openmpt_module_select_subsong, openmpt_module_set_position_order_row};
use serenity::all::{CommandInteraction, CommandOptionType, Context, CreateCommandOption, ResolvedOption, ResolvedValue};
use serenity::builder::CreateCommand;

use crate::botdata::BotDataKey;
use crate::misc::respond_command;

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let data_lock = ctx.data.read().await;
    let session_data_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());
    if session_data_u.is_none() {
        respond_command(&ctx, interaction, "The bot must be in a voice channel").await;
        return;
    }
    drop(data_lock);

    if let ResolvedValue::SubCommand(sub_options) = &interaction.data.options()[0].value {
        match sub_options[0].name {
            "order" => handle_order(ctx, interaction, sub_options).await,
            "subsong" => handle_subsong(ctx, interaction, sub_options).await,
            &_ => { respond_command(&ctx, interaction, "Something has gone horribly wrong").await; return; }
        }
    }
}

pub async fn handle_order(ctx: Context, interaction: &CommandInteraction, options: &Vec<ResolvedOption<'_>>) {
    let data_lock = ctx.data.read().await;
    let session_data_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());

    let order_u = options.iter()
        .find(|option| option.name == "order")
        .unwrap().clone()
        .value;
    let ResolvedValue::Integer(order) = order_u else { unreachable!() };

    let session_data = session_data_u.unwrap().clone();
    let session_lock = session_data.write().await;

    if let Some(current_module) = &session_lock.current_module {
        unsafe { openmpt_module_set_position_order_row(current_module.module.0, order as i32, 0) };
        let current_order = unsafe { openmpt_module_get_current_order(current_module.module.0) };
        if current_order != (order as i32) {
            drop(session_lock);
            respond_command(&ctx, interaction, "The specified order number is out of range").await;
            return;
        }
        drop(session_lock);
        respond_command(&ctx, interaction, &("Jumped to order ".to_string()+&order.to_string())).await;
        return;
    } else {
        drop(session_lock);
        respond_command(&ctx, interaction, "No module is currently playing").await;
        return;
    }
}

pub async fn handle_subsong(ctx: Context, interaction: &CommandInteraction, options: &Vec<ResolvedOption<'_>>) {
    let data_lock = ctx.data.read().await;
    let session_data_u = data_lock.get::<BotDataKey>().unwrap()
        .sessions.get(&interaction.guild_id.unwrap());

    let subsong_u = options.iter()
        .find(|option| option.name == "subsong")
        .unwrap().clone()
        .value;
    let ResolvedValue::Integer(subsong) = subsong_u else { unreachable!() };

    let session_data = session_data_u.unwrap().clone();
    let session_lock = session_data.write().await;

    if let Some(current_module) = &session_lock.current_module {
        let subsong_status = unsafe { openmpt_module_select_subsong(current_module.module.0, subsong as i32) };
        if subsong_status == 0 {
            drop(session_lock);
            respond_command(&ctx, interaction, "The specified subsong number is out of range").await;
            return;
        }
        drop(session_lock);
        respond_command(&ctx, interaction, &("Jumped to subsong ".to_string()+&subsong.to_string())).await;
        return;
    } else {
        drop(session_lock);
        respond_command(&ctx, interaction, "No module is currently playing").await;
        return;
    }
}

pub fn register() -> CreateCommand {
    let order_subcmd = CreateCommandOption::new(CommandOptionType::SubCommand, "order", "Jump to a specified order")
        .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "order", "Order to jump to").required(true));
    let subsong_subcmd = CreateCommandOption::new(CommandOptionType::SubCommand, "subsong", "Jump to a specified subsong")
        .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "subsong", "Subsong to jump to").required(true));

    CreateCommand::new("jump").description("Jump to a specified order in currently playing module")
        .add_option(order_subcmd)
        .add_option(subsong_subcmd)
}