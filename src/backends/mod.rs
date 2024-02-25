pub mod cloudflare;
pub mod porkbun;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Config {
    Porkbun {
        api_key: String,
        secret_key: String,
        domain: String,
        update_every_seconds: u64,
    },
    Cloudflare {
        zone_id: String,
        api_key: String,
        domain: String,
        subdomain: String,
        update_every_seconds: u64,
    },
}

#[derive(Debug, Default)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub record_type: String,
    pub content: String,
}
