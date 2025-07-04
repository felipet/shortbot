// Copyright 2024-2025 Felipe Torres González
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! Library of the ShortBot crate.

use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    types::MessageId,
    utils::command::BotCommands,
};

pub mod configuration;
pub mod errors;
pub mod shortcache;
pub mod telemetry;

pub use errors::{DbError, UserError};
pub use shortcache::ShortCache;

// Bring all the endpoints to the main context.
pub mod endpoints {
    mod default;
    mod help;
    mod liststocks;
    mod receivestock;
    mod settings;
    mod start;
    mod support;

    pub use default::default;
    pub use help::help;
    pub use liststocks::list_stocks;
    pub use receivestock::receive_stock;
    pub use settings::{settings, settings_callback};
    pub use start::start;
    pub use support::support;
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
    Settings {
        msg_id: MessageId,
    },
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
    #[command(description = "Show support information")]
    Support,
    #[command(description = "Show settings menu")]
    Settings,
    #[command(description = "Show available subscription plans")]
    Plans,
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
    #[command(description = "Mostrar información de apoyo")]
    Apoyo,
    #[command(description = "Mostrar la configuración")]
    Configuracion,
    #[command(description = "Mostrar los planes de subscripción")]
    Planes,
}

pub mod users {
    //! Library that includes all the logic related to the management of clients of the bot.
    //!
    //! # Description
    //!
    //! This library includes modules that are meant to implement all the logic related to the management of users of
    //! the bot (clients).
    //!
    //! Clients' metadata is stored in a data base. However, to speed-up the application a cache subsystem has been
    //! developed to keep a coherent copy of the clients' data base in main memory. The application expects no more
    //! than hundreds of users, so keeping their metadata in main memory won't demand a lot of resources.
    //!
    //! How the data is stored is transparent to the bot, which only needs to interface with the module
    //! [crate::ClientHandler]. This is the API to access all the logic related to client's management.
    //!
    //! A minimal setup is required by the startup code. A cache needs to be created at startup and all the handlers
    //! need to configured. This process is automated using [crate::ClientObjectsBuilder].
    //!
    //! ### Example of Use
    //!
    //! TODO: add an example with the setup of the client management subsystem.
    //!
    //! ## Organisation
    //!
    //! The crate includes two main modules:
    //!
    //! 1. [crate::cache] which is in charge of the cache subsystem.
    //! 2. [crate::client] which is in charge of the management logic to keep metadata related to clients.
    //!
    //! ## What Is a Client of the Bot
    //!
    //! Users of the bot become _clients_ when they start using some of the advanced features of the bot. This means
    //! regular users don't get fully registered in the client DB. All the advanced features relate to those features that
    //! need some sort of memory storage.
    //!
    //! The main purpose of the crate is to free the bot's logic of all the stuff related to remember what tickers is a
    //! client subscribed at, or if a client expects to receive some sort of periodical information, and so on.
    //!
    //! Anyway, all the users that happen to use the bot, at least once, get registered. The main purpose of this feature
    //! is enabling later analysis of the bot's usage and how many users are actively using the bot.
    //! So any user that uses the bot gets _soft-registered_ or _auto-registered_. Users become _hard-registered_ when
    //! they start using advanced features.
    //!
    //! ## Why This Is a Separated Crate?
    //!
    //! This crate splits all the logic that relies on the MariaDB backend. The main purpose of this separation
    //! is to enable SQLx to properly analyze and build the queries of the application. The bot features
    //! several DB backends, which is not supported by SQLx as of today.
    //!
    //! The most straightforward workaround is to split the code into several crates, each one of them connects to
    //! a specific DB backend. This way, SQLx can analyze the code and build the queries properly.
    //!
    //! All the code related to handling client's preferences, subscriptions or any other information related to them
    //! is included in this crate as it relies on the MariaDB backend.
    //!
    //! # How To Develop This Library
    //!
    //! In order to build successfully all the code of the application, the following procedure must be followed:
    //!
    //! For each crate of the workspace:
    //!
    //! 1. Set up the environment variables for connecting to the DB backend, either via `export DATABASE_URL` or using
    //!    `.env` files.
    //! 2. Build the crate using `cargo build`.
    //! 3. Run `cargo sqlx prepare` to generate the SQLx prepared queries.
    //!
    //! Remember to commit those files to the repository.
    //!
    //! After that, the whole workspace can be built using `cargo build`, but we need to run SQLx in offline mode:
    //! `export SQLX_OFFLINE=true`.

    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    pub mod subscriptions;
    pub mod user_handler;
    pub mod user_meta;

    pub use subscriptions::Subscriptions;
    pub use user_handler::UserHandler;
    pub use user_meta::UserMeta;

    /// This enum represents the access level of an user of the bot.
    ///
    /// # Description
    ///
    /// The access level is used to determine the level of access to the bot's features for each user.
    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum BotAccess {
        #[default]
        Free,
        Limited,
        Unlimited,
        Admin,
    }

    impl FromStr for BotAccess {
        type Err = &'static str;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "free" => Ok(BotAccess::Free),
                "limited" => Ok(BotAccess::Limited),
                "unlimited" => Ok(BotAccess::Unlimited),
                "admin" => Ok(BotAccess::Admin),
                _ => Err("Invalid BotAccess type"),
            }
        }
    }

    impl std::fmt::Display for BotAccess {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BotAccess::Free => write!(f, "✍️ Free plan"),
                BotAccess::Limited => write!(f, "👷 Limited plan"),
                BotAccess::Unlimited => write!(f, "🥷 Unlimited plan"),
                BotAccess::Admin => write!(f, "💪 Admin"),
            }
        }
    }
}
