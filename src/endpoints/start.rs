//! Handler for the /start command.

use crate::HandlerResult;
use teloxide::prelude::*;
use tracing::info;

/// Start handler.
#[tracing::instrument(
    name = "Start handler",
    skip(bot, msg),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn start(bot: Bot, msg: Message) -> HandlerResult {
    info!("Command /start called");

    let client_name = get_client_name(&msg);

    let messages = vec![
        format!("Welcome {} to the Ibex35 ShortBot!", client_name,),
        "Type /help to see the Bot's help message.".to_owned(),
    ];

    for message in messages {
        bot.send_message(msg.chat.id, message).await?;
    }

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
