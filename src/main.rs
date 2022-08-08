use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = get_configuration().expect("Failed to read config");
    let connection_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect database");

    let listener = TcpListener::bind("127.0.0.1:8080")?;

    startup::run(listener, connection_pool)?.await
}
