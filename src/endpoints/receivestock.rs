//! Handler that lists all the available stocks to the client.

use crate::finance::CNMVProvider;
use crate::finance::Ibex35Market;
use crate::{HandlerResult, ShortBotDialogue};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::{debug, info};

#[tracing::instrument(
    name = "Receive stock handler",
    skip(bot, dialogue, stock_market, q),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn receive_stock(
    bot: Bot,
    dialogue: ShortBotDialogue,
    stock_market: Arc<Ibex35Market>,
    q: CallbackQuery,
) -> HandlerResult {
    if let Some(ticker) = &q.data {
        bot.send_message(
            dialogue.chat_id(),
            format!(
                "You chose the Ibex35 company: <b>{}</b>\nChecking alive short positions...",
                stock_market.stock_by_ticker(ticker).unwrap().name()
            ),
        )
        .parse_mode(ParseMode::Html)
        .await?;
        info!("Selected stock: {}", ticker);
    } else {
        bot.send_message(dialogue.chat_id(), "No stock given")
            .await?;
        info!("No valid ticker was received");
    }

    let provider = CNMVProvider::new();
    let stock_object = stock_market.stock_by_ticker(&q.data.unwrap()[..]).unwrap();
    debug!("Stock descriptor: {stock_object}");
    let positions = provider.short_positions(stock_object).await;
    debug!("Received AliveShortPositions: {:?}", positions);

    if positions.is_ok() {
        let shorts = positions.unwrap();
        bot.send_message(
            dialogue.chat_id(),
            format!(
                "The total of short positions is: <b>{:.2} %</b>",
                shorts.total
            ),
        )
        .parse_mode(ParseMode::Html)
        .await?;
        bot.send_message(dialogue.chat_id(), format!("{}", shorts))
            .parse_mode(ParseMode::Html)
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "Information not available")
            .await?;
    }

    info!("Short position request served");
    dialogue.exit().await?;

    Ok(())
}
