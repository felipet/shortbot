//! Handler that lists all the available stocks to the client.

use crate::finance::Ibex35Market;
use crate::{HandlerResult, ShortBotDialogue, State};
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};
use tracing::{info, trace};

#[tracing::instrument(
    name = "List stocks handler",
    skip(bot, dialogue, msg, stock_market),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn list_stocks(
    bot: Bot,
    dialogue: ShortBotDialogue,
    msg: Message,
    stock_market: Arc<Ibex35Market>,
) -> HandlerResult {
    info!("Command /ChooseStock called");

    let market = stock_market.list_tickers();
    trace!(
        "The available tickers in the {} market are: {:?}",
        stock_market.market_name(),
        market
    );

    // Present the tickers in a table with 4 columns to reduce the number of rows.
    let mut i: usize = 0;
    let rows = 4;
    let mut keyboard_markup = InlineKeyboardMarkup::new([vec![
        InlineKeyboardButton::callback::<&str, &str>(market[i].as_ref(), market[i].as_ref()),
        InlineKeyboardButton::callback::<&str, &str>(
            market[i + 1].as_ref(),
            market[i + 1].as_ref(),
        ),
        InlineKeyboardButton::callback::<&str, &str>(
            market[i + 2].as_ref(),
            market[i + 2].as_ref(),
        ),
        InlineKeyboardButton::callback::<&str, &str>(
            market[i + 3].as_ref(),
            market[i + 3].as_ref(),
        ),
    ]]);
    let stock_len = market.len();
    i += rows;

    while i < stock_len - rows {
        keyboard_markup = keyboard_markup.append_row(vec![
            InlineKeyboardButton::callback::<&str, &str>(market[i].as_ref(), market[i].as_ref()),
            InlineKeyboardButton::callback::<&str, &str>(
                market[i + 1].as_ref(),
                market[i + 1].as_ref(),
            ),
            InlineKeyboardButton::callback::<&str, &str>(
                market[i + 2].as_ref(),
                market[i + 2].as_ref(),
            ),
            InlineKeyboardButton::callback::<&str, &str>(
                market[i + 3].as_ref(),
                market[i + 3].as_ref(),
            ),
        ]);

        i += rows;
    }

    if stock_len % 4 != 0 {
        while i < stock_len {
            keyboard_markup = keyboard_markup.append_to_row(
                stock_len / rows + 1,
                InlineKeyboardButton::callback::<&str, &str>(
                    market[i].as_ref(),
                    market[i].as_ref(),
                ),
            );

            i += 1;
        }
    }

    bot.send_message(msg.chat.id, "Select a stock ticker:")
        .reply_markup(keyboard_markup)
        .await?;

    info!("Stocks listed, moving to State::ReceiveStock");

    dialogue.update(State::ReceiveStock).await?;

    Ok(())
}
