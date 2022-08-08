use sqlx::{Connection, PgConnection, PgPool, Executor};
use std::net::TcpListener;
use zero2prod::{configuration::{get_configuration, DatabaseSettings}, startup, telemetry};
use uuid::Uuid;
use once_cell::sync::Lazy;

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber_name = "test".to_string();
    let default_layer_filter = "info".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = telemetry::get_subscriber(subscriber_name, default_layer_filter, std::io::stdout);
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber = telemetry::get_subscriber(subscriber_name, default_layer_filter, std::io::sink);
        telemetry::init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

// launch our app in background
async fn start_server() -> TestApp {
    Lazy::force(&TRACING);

    let mut config = get_configuration().expect("Failed to read configuration");
    config.database.database_name = Uuid::new_v4().to_string();

    let connection = configure_database(&config.database).await;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");

    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let server = startup::run(listener, connection.clone()).expect("Failed to bind address");

    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let query = format!(r#"CREATE DATABASE "{}""#, config.database_name);
    let connection_string = config.connection_string_without_db();

    println!("{} on {}", query, connection_string);

    let mut connection = PgConnection::connect("postgres://federicovitale@localhost:5432/")
        .await
        .expect("Failed to connect to the database");

    connection.execute(format!(r#"CREATE DATABASE "{}""#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    let pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed t oconnect to postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = start_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = start_server().await;
    let client = reqwest::Client::new();

    let body = "name=Federico&email=mail%40fedevitale.dev";
    let response = client
        .post(&format!("{}/subscriptions", app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions");

    assert_eq!(saved.email, "mail@fedevitale.dev");
    assert_eq!(saved.name, "Federico");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    let app = start_server().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=federico", "missing email"),
        ("email=user%40gmail.com", "missing name"),
        ("email=user&name=john", "invalid email address"),
        ("", "missing both email and name"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 when the payload was {}",
            error_message
        );
    }
}
