use backends::configuration::Config;
use backends::configuration::Record;

use reqwest::Client;
use serde_json::{from_str, Value};

struct DDNSClient {
    client: Client,
    config: Config,
}

impl DDNSClient {
    async fn new(config_path: &String) -> Self {
        let client = Client::new();
        let file = std::fs::read_to_string(config_path).unwrap_or_else(|err| {
            println!("Error reading file: {}", err);
            std::process::exit(1);
        });

        let config: Config = from_str(&file).unwrap();
        Self { client, config }
    }

    async fn get_ip(&self) -> String {
        let response = self
            .client
            .get("https://api.ipify.org")
            .send()
            .await
            .unwrap();
        let body = response.text().await.unwrap();
        let ip = from_str::<Value>(&body).unwrap();
        ip.as_str().unwrap().to_string()
    }

    async fn retrieve_record(&self) -> Option<Record> {
        match &self.config {
            Config::Porkbun { .. } => None,
            Config::Cloudflare {
                domain,
                zone_id,
                api_key,
            } => {
                return backends::cloudflare::retrieve_record(
                    &self.client,
                    &domain,
                    &zone_id,
                    &api_key,
                )
                .await;
            }
        }
    }

    async fn update_record(&self) -> Option<Record> {
        Some(Record::default())
    }
}

#[tokio::main]
async fn main() -> () {
    let arguments = std::env::args().collect::<Vec<String>>();

    let path = arguments.get(1).unwrap_or_else(|| {
        println!("Usage: ddns-rs <config_path>");
        std::process::exit(1);
    });

    let client = DDNSClient::new(&path).await;

    let current_record = client.retrieve_record().await;
    let current_ip = current_record.unwrap_or_default().content;

    loop {
        let new_ip = client.get_ip().await;
        if new_ip != current_ip {
            current_ip = new_ip;
            client.update_record().await;
        }

        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
