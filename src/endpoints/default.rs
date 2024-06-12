//! Handler for the /help command.

use crate::HandlerResult;
use teloxide::{prelude::*, types::ParseMode};
use tracing::{debug, info};

/// Help handler.
#[tracing::instrument(
    name = "Default handler",
    skip(bot, msg, update),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn default(bot: Bot, msg: Message, update: Update) -> HandlerResult {
    info!("Garbage sent");

    // First, try to retrieve the user of the chat.
    let lang_code = match update.user() {
        Some(user) => user.language_code.clone(),
        None => None,
    };

    debug!("The user's language code is: {:?}", lang_code);

    let message = match lang_code.as_deref().unwrap_or("en") {
        "es" => _warning_es(),
        _ => _warning_en(),
    };

    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

fn _warning_es() -> String {
    include_str!("../../data/templates/warning_es.txt").to_owned()
}

fn _warning_en() -> String {
    include_str!("../../data/templates/warning_en.txt").to_owned()
}
