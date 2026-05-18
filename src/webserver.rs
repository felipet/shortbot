// Copyright 2026 Felipe Torres González
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

//! Module with the logic for the initialization of the Axum web server.

use crate::{
    UPDATE_BUFFER_SIZE, UserHandler, configuration::Settings, endpoints::webhook,
    errors::ServiceError,
};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use metrics_exporter_prometheus::PrometheusHandle;
use secrecy::SecretString;
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use teloxide::{Bot, adaptors::Throttle, update_listeners::UpdateListener};
use tokio::{
    net::TcpListener,
    sync::mpsc::{self, Receiver, Sender},
};

/// Shared state for handlers of the Axum web server.
#[derive(Clone)]
pub struct WebServerState {
    pub user_handler: Arc<UserHandler>,
    pub bot: Throttle<Bot>,
    pub webhook_token: SecretString,
    pub update_buffer_tx: Sender<String>,
}

pub fn setup_webserver(
    user_handler: Arc<UserHandler>,
    bot: Throttle<Bot>,
    webhook_token: &SecretString,
    bot_router: Router,
    metrics_handle: PrometheusHandle,
) -> Result<(Router, Receiver<String>), ServiceError> {
    // MPSC channel to trigger short position updates to users with subscriptions.
    let (update_buffer_tx, update_buffer_rx) = mpsc::channel::<String>(UPDATE_BUFFER_SIZE);

    // Build an Axum HTTP server.
    let state = WebServerState {
        user_handler,
        bot,
        webhook_token: webhook_token.clone(),
        update_buffer_tx,
    };
    let webook_router = Router::new()
        .route("/webhook", post(webhook::webhook_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            webhook::auth_client,
        ));

    let main_router = Router::new()
        .route(
            "/metrics",
            get(move || async move { metrics_handle.render() }),
        )
        .merge(webook_router)
        .with_state(state);

    // Launch the Axum server.
    let app = axum::Router::new()
        .nest("/adm", main_router)
        .fallback_service(bot_router);

    Ok((app, update_buffer_rx))
}

pub async fn setup_bot_router(
    bot: Throttle<Bot>,
    settings: &Settings,
) -> Result<
    (
        impl UpdateListener<Err = std::convert::Infallible> + use<>,
        impl std::future::Future<Output = ()> + Send + use<>,
        TcpListener,
        Router,
    ),
    ServiceError,
> {
    // Build a listener based on the axum server.
    let http_server_address = SocketAddr::from_str(&format!(
        "{}:{}",
        &settings.application.http_server_host, settings.application.http_server_port
    ))
    .expect("Failed to build a socket using the configuration");

    let tcp_listener = TcpListener::bind(http_server_address)
        .await
        .expect("Failed to bind to the provided address");

    let (listener, stop_future, bot_router) = teloxide::update_listeners::webhooks::axum_to_router(
        bot,
        teloxide::update_listeners::webhooks::Options::new(
            http_server_address,
            format!(
                "{}{}",
                settings.application.webhook_url, settings.application.webhook_path
            )
            .parse()
            .unwrap(),
        ),
    )
    .await
    .map_err(|e| ServiceError::BotInitError(e.to_string()))?;

    Ok((listener, stop_future, tcp_listener, bot_router))
}
