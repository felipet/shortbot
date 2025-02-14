// Copyright 2024 Felipe Torres González
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
    utils::command::BotCommands,
};

pub mod configuration;
pub mod telemetry;

// Bring all the endpoints to the main context.
pub mod endpoints {
    mod default;
    mod help;
    mod liststocks;
    mod receivestock;
    mod start;
    mod support;

    pub use default::default;
    pub use help::help;
    pub use liststocks::list_stocks;
    pub use receivestock::receive_stock;
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
}
