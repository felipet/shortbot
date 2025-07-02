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
//    limitations under the License.

//! Custom error types.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("missing stock information in the DB")]
    MissingStockInfo(String),
    #[error("unknown db error")]
    Unknown(String),
    #[error("unknown error from the QuestDB dataserver")]
    UnknownQdb(String),
    #[error("error from Valkey DB server")]
    UnknownValkey(String),
}

#[derive(Error, Debug)]
pub enum UserError {
    #[error("Wrong subscription string format")]
    WrongSubscriptionString(String),
    #[error("The user ID is not registered")]
    ClientNotRegistered,
    #[error("Subscription limit reached")]
    ClientLimitReached,
    #[error("serialisation error")]
    SerialisationError(String),
}
