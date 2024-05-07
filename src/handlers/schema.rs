//! Main handler of the ShortBot.
//!
//! # Description
//!
//! The handler implemented herein shall be passed to the [Teloxide::Despatcher::builder]
//! instance of the main application.
//! All valid combinations of Messages and States shall be contemplated in the implementation
//! of this handler.

use crate::{
    endpoints::{help, start},
    CommandEng, State,
};
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
            .branch(case![CommandEng::Help].endpoint(help)),
    );

    let message_handler = Update::filter_message().branch(command_handler);

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(message_handler)
}
