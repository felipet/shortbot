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

use tracing::{Level, level_filters::LevelFilter};
use tracing_subscriber::{Layer, filter::Targets, fmt, prelude::*};

pub fn configure_tracing(tracing_level: &str) {
    // Set the tracing logic.
    let (tracing_level, tracing_levelfilter) = match tracing_level {
        "info" => (Level::INFO, LevelFilter::INFO),
        "debug" => (Level::DEBUG, LevelFilter::DEBUG),
        "warn" => (Level::WARN, LevelFilter::WARN),
        "error" => (Level::ERROR, LevelFilter::ERROR),
        _ => (Level::TRACE, LevelFilter::TRACE),
    };

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_filter(tracing_levelfilter),
        )
        .with(
            Targets::new()
                .with_target("bot_core", tracing_level)
                .with_target("clientlib", tracing_level),
        )
        .init();
}
