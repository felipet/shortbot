//! Main handler of the ShortBot.
//!
//! # Description
//!
//! The handler implemented herein shall be passed to the [Teloxide::Despatcher::builder]
//! instance of the main application.
//! All valid combinations of Messages and States shall be contemplated in the implementation
//! of this handler.

use crate::{endpoints::*, CommandEng, State};
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
};

/// Main handler of the ShortBot application.
pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<CommandEng, _>().branch(
        case![State::Start]
            .branch(case![CommandEng::Start].endpoint(start))
            .branch(case![CommandEng::Help].endpoint(help))
            .branch(case![CommandEng::ChooseStock].endpoint(list_stocks)),
    );

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ListStocks].endpoint(list_stocks));

    let query_handler =
        Update::filter_callback_query().branch(case![State::ReceiveStock].endpoint(receive_stock));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(query_handler)
}
