use std::sync::Arc;

use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use tokio::sync::Mutex;

pub mod model;
pub mod trello;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub key: String,
    pub token: String,
}

lazy_static::lazy_static! {
    static ref CREDS: Arc<Mutex<Option<Credentials>>> = Arc::new(Mutex::new(None));
    static ref HTTP: ClientWithMiddleware = build_client();
}

fn build_client() -> ClientWithMiddleware {
    let client = Client::builder()
        .user_agent(format!("rbxtrello/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client");

    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);

    ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

pub fn http() -> &'static ClientWithMiddleware {
    &HTTP
}

pub async fn set_credentials(key: String, token: String) {
    let mut guard = CREDS.lock().await;
    *guard = Some(Credentials { key, token });
}

pub async fn credentials() -> anyhow::Result<Credentials> {
    let guard = CREDS.lock().await;
    guard
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Trello credentials not set"))
}
