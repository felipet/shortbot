use tracing::{
    subscriber::{set_global_default, Subscriber},
    Level,
};
use tracing_subscriber::FmtSubscriber;

pub fn get_subscriber(tracing_level: &str) -> impl Subscriber + Send + Sync {
    // Set the tracing logic.
    let tracing_level = match tracing_level {
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing_level)
        .finish();
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    set_global_default(subscriber).expect("Failed to set subscriber.");
}
