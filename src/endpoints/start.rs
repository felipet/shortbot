//! Handler for the /start command.

use crate::HandlerResult;
use teloxide::prelude::*;
use tracing::{debug, info};

/// Start handler.
#[tracing::instrument(
    name = "Start handler",
    skip(bot, msg, update),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn start(bot: Bot, msg: Message, update: Update) -> HandlerResult {
    info!("Command /start requested");

    let client_name = get_client_name(&msg);

    // Let's ry to retrieve the user of the chat.
    let lang_code = match update.user() {
        Some(user) => user.language_code.clone(),
        None => None,
    };

    debug!("The user's language code is: {:?}", lang_code);

    let message = match lang_code {
        Some(lang_code) => match lang_code.as_str() {
            "es" => _start_es(&client_name),
            _ => _start_en(&client_name),
        },
        _ => _start_en(&client_name),
    };

    bot.send_message(msg.chat.id, message).await?;

    Ok(())
}

/// Get a human-friendly identifier for the client of the chat.
fn get_client_name(msg: &Message) -> String {
    if let Some(name) = msg.chat.first_name() {
        String::from(name)
    } else {
        match msg.chat.username() {
            Some(username) => String::from(username),
            // When no identifier is accessible for the client, call it "investor".
            None => String::from("investor"),
        }
    }
}

/// Start handler (English version).
fn _start_en(username: &str) -> String {
    format!(
        include_str!("../../data/templates/welcome_en.txt"),
        username,
    )
}

/// Start handler (Spanish version).
fn _start_es(username: &str) -> String {
    format!(
        include_str!("../../data/templates/welcome_es.txt"),
        username,
    )
}
