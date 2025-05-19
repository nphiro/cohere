use std::{thread::sleep, time::Duration};

use cohere::{env, instrument, secure, server};
use rand::Rng;

#[derive(serde::Deserialize, Debug, Default)]
struct Config {
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _inst_guard = instrument::init("github.com/nphiro", "cohere")?;

    let mut config = Config::default();

    env::parse(&mut config);

    println!("URL: {}", config.url);

    tracing::info!(config.url, "Will this work?");
    let span = tracing::info_span!("Starting application");
    let _enter = span.enter();
    tracing::info!(config.url, "Hello, world!");
    tracing::info!("Adding numbers");
    add(get_random_number(), get_random_number());

    match secure::validate_totp("JBSWY3DPEHPK3PXP", "836896", 30) {
        Ok(()) => println!("Valid TOTP"),
        Err(e) => println!("Invalid TOTP: {}", e),
    }

    server::serve_http(server::new_http(), 8000).await?;

    Ok(())
}

#[tracing::instrument]
fn add(a: i32, b: i32) -> i32 {
    tracing::info!(test = 4, "Adding {} and {}", a, b);
    sleep(Duration::from_secs(3));
    a + b
}

#[tracing::instrument(fields(custom.label = "test"))]
fn get_random_number() -> i32 {
    let mut rng = rand::rng();
    rng.random_range(1..=100)
}
