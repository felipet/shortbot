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

//! Main handler of the ShortBot.
//!
//! # Description
//!
//! The handler implemented herein shall be passed to the [Teloxide::Despatcher::builder]
//! instance of the main application.
//! All valid combinations of Messages and States shall be contemplated in the implementation
//! of this handler.

use crate::{CommandEng, CommandSpa, State, endpoints::*};
use teloxide::{
    dispatching::{UpdateHandler, dialogue, dialogue::InMemStorage},
    prelude::*,
};

/// Main handler of the ShortBot application.
pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler_eng = teloxide::filter_command::<CommandEng, _>().branch(
        case![State::Start]
            .branch(case![CommandEng::Start].endpoint(start))
            .branch(case![CommandEng::Help { section }].endpoint(help))
            .branch(case![CommandEng::Short].endpoint(list_stocks))
            .branch(case![CommandEng::Support].endpoint(support))
            .branch(case![CommandEng::Settings].endpoint(settings))
            .branch(case![CommandEng::Subscriptions].endpoint(subscriptions_menu))
            .branch(case![CommandEng::Brief].endpoint(show_subscriptions))
            .branch(case![CommandEng::Plans].endpoint(plans)),
    );

    let command_handler_spa = teloxide::filter_command::<CommandSpa, _>().branch(
        case![State::Start]
            .branch(case![CommandSpa::Inicio].endpoint(start))
            .branch(case![CommandSpa::Ayuda { section }].endpoint(help))
            .branch(case![CommandSpa::Short].endpoint(list_stocks))
            .branch(case![CommandSpa::Apoyo].endpoint(support))
            .branch(case![CommandSpa::Configuracion].endpoint(settings))
            .branch(case![CommandSpa::Subscripciones].endpoint(subscriptions_menu))
            .branch(case![CommandSpa::Resumen].endpoint(show_subscriptions))
            .branch(case![CommandSpa::Planes].endpoint(plans)),
    );

    let message_handler = Update::filter_message()
        .branch(command_handler_eng)
        .branch(command_handler_spa)
        .branch(case![State::ListStocks].endpoint(list_stocks))
        .endpoint(default);

    let query_handler = Update::filter_callback_query()
        .branch(case![State::ListStocksByName { msg_id }].endpoint(list_stock_by_name))
        .branch(case![State::ReceiveStock { msg_id }].endpoint(receive_stock))
        .branch(case![State::Settings { msg_id }].endpoint(settings_callback))
        .branch(case![State::Subscriptions { msg_id }].endpoint(subscriptions_callback))
        .branch(case![State::AddSubscriptions { msg_id }].endpoint(subscriptions_callback))
        .branch(case![State::DeleteSubscriptions { msg_id }].endpoint(subscriptions_callback))
        .branch(case![State::LanguageSelection { msg_id }].endpoint(language_selection_callback));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(query_handler)
}
