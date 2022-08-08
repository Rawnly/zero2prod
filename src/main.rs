//! src/main.rs
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup;
use sqlx::PgPool;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read config");
    let connection_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect database");

    let address = format!("127.0.0.1:{}", config.port); let listener = TcpListener::bind(&address)?;

    tracing::info!("Server running at: {}", address);
    startup::run(listener, connection_pool)?.await
}
