//! Handler for the /support command.

use crate::HandlerResult;
use teloxide::{prelude::*, types::ParseMode};
use tracing::{debug, info};

/// Support handler.
#[tracing::instrument(
    name = "Support handler",
    skip(bot, msg, update),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn support(bot: Bot, msg: Message, update: Update) -> HandlerResult {
    info!("Command /support requested");

    // First, try to retrieve the user of the chat.
    let lang_code = match update.user() {
        Some(user) => user.language_code.clone(),
        None => None,
    };

    debug!("The user's language code is: {:?}", lang_code);

    let message = match lang_code {
        Some(lang_code) => match lang_code.as_str() {
            "es" => _support_es(),
            _ => _support_en(),
        },
        _ => _support_en(),
    };

    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::Html)
        .disable_web_page_preview(true)
        .await?;

    Ok(())
}

/// Support handler (English version).
fn _support_en() -> String {
    include_str!("../../data/templates/support_en.txt").to_string()
}

/// Support handler (Spanish version).
fn _support_es() -> String {
    include_str!("../../data/templates/support_es.txt").to_string()
}
