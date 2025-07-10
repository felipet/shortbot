// Copyright 2025 Felipe Torres González
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

//! Module that contains definitions related to configuration parameters for users of the bot.

use serde::{Deserialize, Serialize};

/// Configuration parameters of a bot's user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserConfig {
    pub show_broadcast_msg: bool,
    pub prefer_tickers: bool,
}

impl Default for UserConfig {
    fn default() -> Self {
        UserConfig {
            show_broadcast_msg: true,
            prefer_tickers: true,
        }
    }
}
