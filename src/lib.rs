//! Library of the ShortBot crate.

use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    utils::command::BotCommands,
};

pub mod configuration;
pub mod telemetry;

/// Name of the data file that contains the descriptors for the Ibex35 companies.
pub const IBEX35_STOCK_DESCRIPTORS: &str = "ibex35.toml";

// Bring all the endpoints to the main context.
pub mod endpoints {
    mod help;
    mod liststocks;
    mod receivestock;
    mod start;

    pub use help::help;
    pub use liststocks::list_stocks;
    pub use receivestock::receive_stock;
    pub use start::start;
}

// Bring all the handlers to the main context.
pub mod handlers {
    mod schema;

    pub use schema::*;
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

type ShortBotDialogue = Dialogue<State, InMemStorage<State>>;

/// State machine
///
/// # Description
///
/// TODO! Document the state machine states.
#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ListStocks,
    ReceiveStock,
}

/// User commands in English language
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "camelCase",
    description = "These commands are supported by the Bot:"
)]
pub enum CommandEng {
    #[command(description = "Start a new session")]
    Start,
    #[command(description = "Display help message")]
    Help,
    #[command(description = "Check short position of a stock")]
    Short,
}

/// User commands in Spanish language
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "camelCase",
    description = "Estos son los comandos soportados por el Bot:"
)]
pub enum CommandSpa {
    #[command(description = "Iniciar una nueva sesión")]
    Inicio,
    #[command(description = "Mostrar la ayuda")]
    Ayuda,
    #[command(description = "Consultar posiciones de una acción")]
    Short,
}

/// Finance module.
///
/// # Description
///
/// This module includes all the logic related to extract and process financial data.
pub mod finance {
    mod cnmv_scrapper;
    mod ibex35;
    mod ibex_company;

    use core::fmt;

    pub use cnmv_scrapper::CNMVProvider;
    pub use ibex35::{load_ibex35_companies, Ibex35Market};
    pub use ibex_company::IbexCompany;

    use date::Date;

    /// Short position descriptor.
    #[derive(Debug)]
    pub struct ShortPosition {
        /// This is the name of the investment fund that owns the short position.
        pub owner: String,
        /// This is a percentage over the company's total capitalization that indicates
        /// the amount of shares sold in short by the owner against the value of the
        /// company.
        pub weight: f32,
        /// Date in which the short position was stated.
        pub date: String,
    }

    impl fmt::Display for ShortPosition {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} - {} ({})", self.owner, self.weight, self.date)
        }
    }

    /// Container of active short positions of a company.
    ///
    /// # Description
    ///
    /// This `struct` gathers all the active short positions of a company. It is alike to
    /// the table shown in the web page when checking for the short positions of a company.
    ///
    /// Short positions are stated once per day, no later than 15:30. Thus a full timestamp
    /// is not really useful. Only the date is kept for the entries.
    #[derive(Debug)]
    pub struct AliveShortPositions {
        /// Summation of all the active [ShortPosition::weight] of the company.
        pub total: f32,
        /// Collection of active [ShortPosition] for a company.
        pub positions: Vec<ShortPosition>,
        /// Timestamp of the active positions.
        pub date: Date,
    }

    impl AliveShortPositions {
        /// Constructor of the [AliveShortPositions] class.
        pub fn new() -> AliveShortPositions {
            AliveShortPositions {
                total: 0.0,
                positions: Vec::new(),
                date: Date::today_utc(),
            }
        }
    }

    impl Default for AliveShortPositions {
        fn default() -> Self {
            Self::new()
        }
    }

    impl fmt::Display for AliveShortPositions {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for position in self.positions.iter() {
                writeln!(
                    f,
                    "✓ {}: <b>{} %</b> ({})",
                    position.owner.as_str(),
                    position.weight,
                    position.date
                )?;
            }

            Ok(())
        }
    }
}
