//! Handler for the /help command.

use crate::{CommandEng, HandlerResult};
use teloxide::{prelude::*, utils::command::BotCommands};
use tracing::info;

/// Help handler.
#[tracing::instrument(
    name = "Help handler",
    skip(bot, msg),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    info!("Command /help requested");
    bot.send_message(msg.chat.id, CommandEng::descriptions().to_string())
        .await?;

    Ok(())
}
