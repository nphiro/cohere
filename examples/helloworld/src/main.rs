use std::{thread::sleep, time::Duration};

use axum::extract::Path;
use cohere::{
    env, instrument, secure,
    server::{self, add_http_route},
};
use rand::Rng;

#[derive(serde::Deserialize, Debug, Default)]
struct Config {
    url: String,
}

fn main() {
    let _inst_guard = instrument::init("github.com/nphiro", "cohere").unwrap();

    let mut config = Config::default();

    env::parse(&mut config);

    println!("URL: {}", config.url);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tracing::info!(config.url, "will this work?");
            let span = tracing::info_span!("starting application");
            let _enter = span.enter();
            tracing::info!(config.url, "hello, world!");
            tracing::info!("adding numbers");
            add(get_random_number(), get_random_number());

            match secure::validate_totp("JBSWY3DPEHPK3PXP", "836896", 30) {
                Ok(()) => println!("Valid TOTP"),
                Err(e) => println!("Invalid TOTP: {}", e),
            }

            let mut app = server::new_http();

            app = add_http_route(
                app,
                "/example",
                axum::routing::get(|| async { "Hello, World!" }),
            );

            app = add_http_route(
                app,
                "/users/{id}",
                axum::routing::get(|Path(id): Path<String>| async move {
                    format!("Hello, User {}!", id)
                }),
            );

            app = add_http_route(
                app,
                "/add/{a}/{b}",
                axum::routing::get(|Path((a, b)): Path<(i32, i32)>| async move {
                    let sum = add(a, b);
                    format!("Sum: {}", sum)
                }),
            );

            let client = reqwest::Client::new();

            let req = client.get("http://localhost:8000/add/5/10");

            match cohere::client::send_http(req, Some("/add/{a}/{b}")).await {
                Ok(response) => {
                    tracing::info!("Response: {:?}", response);
                }
                Err(err) => {
                    tracing::error!("Error: {:?}", err);
                }
            }

            server::serve_http(app, 8000).await.unwrap();
        });
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
