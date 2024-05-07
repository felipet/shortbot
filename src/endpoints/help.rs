//! Handler for the /help command.

use crate::HandlerResult;
use teloxide::prelude::*;

/// Help handler.
pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "DBG::Help command").await?;

    Ok(())
}
