//! Library of the ShortBot crate.

use teloxide::utils::command::BotCommands;

pub mod configuration;

// Bring all the endpoints to the main context.
pub mod endpoints {
    mod help;
    mod start;

    pub use help::help;
    pub use start::start;
}

// Bring all the handlers to the main context.
pub mod handlers {
    mod schema;

    pub use schema::*;
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// State machine
///
/// # Description
///
/// TODO! Document the state machine states.
#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
}

/// Application commands in English language
///
/// # Description
///
/// TODO! document the commands.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "camelCase")]
pub enum CommandEng {
    /// Start command
    Start,
    /// Help message
    Help,
}
