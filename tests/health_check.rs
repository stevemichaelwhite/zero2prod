use std::{net::TcpListener, sync::Once};

use env_logger::Env;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{DatabaseSettings, get_configuration};
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

static INIT: Once = Once::new();

fn init_logger() {
    INIT.call_once(|| {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    });
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");
    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().into();
    let db_pool = configure_database(&configuration.database).await;
    let server = zero2prod::run(listener, db_pool.clone()).expect("Failed to bind to address.");
    let _ = tokio::spawn(server);
    TestApp { address, db_pool }
}

#[tokio::test]
async fn health_check_works() {
    init_logger();
    let app = spawn_app().await;
    let address = app.address.as_str();
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
    println!("{:?}", response.text().await.unwrap());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    init_logger();
    let app = spawn_app().await;
    let address = app.address.as_str();
    let connection = app.db_pool;
    let client = reqwest::Client::new();

    let body = "name=steve%20white&email=stevemichaelwhite%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to post to subscriptions.");
    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&connection)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "stevemichaelwhite@gmail.com");
    assert_eq!(saved.name, "steve white");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    init_logger();
    let app = spawn_app().await;
    let address = app.address.as_str();
    let _connection = app.db_pool;
    let client = reqwest::Client::new();
    let test_cases = [
        ("name=steve%20white", "missing email"),
        ("email=stevemichaelwhite%40gmail.com", "missing name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", &address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to post to subscriptions.");
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
