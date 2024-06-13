//! Main handler of the ShortBot.
//!
//! # Description
//!
//! The handler implemented herein shall be passed to the [Teloxide::Despatcher::builder]
//! instance of the main application.
//! All valid combinations of Messages and States shall be contemplated in the implementation
//! of this handler.

use crate::{endpoints::*, CommandEng, CommandSpa, State};
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
};

/// Main handler of the ShortBot application.
pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler_eng = teloxide::filter_command::<CommandEng, _>().branch(
        case![State::Start]
            .branch(case![CommandEng::Start].endpoint(start))
            .branch(case![CommandEng::Help].endpoint(help))
            .branch(case![CommandEng::Short].endpoint(list_stocks))
            .branch(case![CommandEng::Support].endpoint(support)),
    );

    let command_handler_spa = teloxide::filter_command::<CommandSpa, _>().branch(
        case![State::Start]
            .branch(case![CommandSpa::Inicio].endpoint(start))
            .branch(case![CommandSpa::Ayuda].endpoint(help))
            .branch(case![CommandSpa::Short].endpoint(list_stocks))
            .branch(case![CommandSpa::Apoyo].endpoint(support)),
    );

    let message_handler = Update::filter_message()
        .branch(command_handler_eng)
        .branch(command_handler_spa)
        .branch(case![State::ListStocks].endpoint(list_stocks))
        .endpoint(default);

    let query_handler =
        Update::filter_callback_query().branch(case![State::ReceiveStock].endpoint(receive_stock));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(query_handler)
}
