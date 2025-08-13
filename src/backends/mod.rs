pub mod cloudflare;
pub mod porkbun;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub update_every_seconds: u64,
    #[serde(flatten)]
    pub backend: BackendConfig,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackendConfig {
    Porkbun {
        api_key: String,
        secret_key: String,
        domain: String,
    },
    Cloudflare {
        zone_id: String,
        api_key: String,
        domain: String,
        subdomain: String,
    },
}

#[derive(Debug, Default)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub record_type: String,
    pub content: String,
}
