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
    info!("Command /choosestock requested");

    let market = stock_market.list_tickers();
    trace!(
        "The available tickers in the {} market are: {:?}",
        stock_market.market_name(),
        market
    );

    // Present the tickers in a table with 5 columns to reduce the number of rows.
    let cols_per_row: usize = 5;
    let stock_len = market.len();

    // Populate the first row
    let mut keyboard_markup =
        InlineKeyboardMarkup::new([vec![InlineKeyboardButton::callback::<&str, &str>(
            market[0].as_ref(),
            market[0].as_ref(),
        )]]);

    for company in market.iter().take(cols_per_row).skip(1) {
        keyboard_markup = keyboard_markup.append_to_row(
            0,
            InlineKeyboardButton::callback::<&str, &str>(company, company),
        );
    }

    // Populate rows by chunks of `cols_per_row` buttons
    for i in 1..(stock_len / cols_per_row) {
        for j in 0..cols_per_row {
            keyboard_markup = keyboard_markup.append_to_row(
                i,
                InlineKeyboardButton::callback::<&str, &str>(
                    market[j + i * cols_per_row].as_ref(),
                    market[j + i * cols_per_row].as_ref(),
                ),
            );
        }
    }

    // Finally, add the remainder in case the number of items is not divisible by `cols_per_row`
    if stock_len % cols_per_row != 0 {
        let mut i = stock_len - cols_per_row;
        while i < stock_len {
            keyboard_markup = keyboard_markup.append_to_row(
                stock_len / cols_per_row + 1,
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
