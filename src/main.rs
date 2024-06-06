//! Main file of the Shortbot

use secrecy::ExposeSecret;
use shortbot::finance::load_ibex35_companies;
use shortbot::{
    configuration::Settings,
    handlers,
    telemetry::{get_subscriber, init_subscriber},
    State, IBEX35_STOCK_DESCRIPTORS,
};
use std::sync::Arc;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the settings.
    let settings = Settings::new().expect("Failed to parse configuration files.");

    // Initialize the tracing subsystem.
    let subscriber = get_subscriber(settings.tracing_level.as_str());
    init_subscriber(subscriber);

    let ibexdata_path = std::path::PathBuf::from(settings.data_path).join(IBEX35_STOCK_DESCRIPTORS);

    let ibex35 = load_ibex35_companies(ibexdata_path.as_os_str().to_str().unwrap())
        .expect("Failed to parse IBEX35 companies.");
    let ibex35 = Arc::new(ibex35);

    info!("Started ShortBot server");

    let bot = Bot::new(settings.application.api_token.expose_secret());

    info!("Dispatching");

    let ibex35_clone = Arc::clone(&ibex35);

    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![ibex35_clone, InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Closed ShortBot server");

    Ok(())
}
