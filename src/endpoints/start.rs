//! Handler for the /start command.

use crate::HandlerResult;
use teloxide::prelude::*;

/// Start handler.
pub async fn start(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "DBG::Start command").await?;

    Ok(())
}
