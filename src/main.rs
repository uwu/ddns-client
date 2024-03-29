use reqwest::Client;
use tokio::time::{sleep, Duration};

pub mod backends;
// pub mod tray;

struct DDNSClient {
    client: Client,
    config: backends::Config,
    timeout: Duration,
}

impl DDNSClient {
    async fn new(config_path: &String) -> Self {
        let client = Client::new();
        let file = std::fs::read_to_string(config_path).unwrap_or_else(|err| {
            println!("Error reading file: {}", err);
            std::process::exit(1);
        });

        let config: backends::Config = serde_json::from_str(&file).unwrap();

        let timeout = Duration::from_secs(match config {
            backends::Config::Porkbun {
                update_every_seconds,
                ..
            } => update_every_seconds,
            backends::Config::Cloudflare {
                update_every_seconds,
                ..
            } => update_every_seconds,
        });

        Self {
            client,
            config,
            timeout,
        }
    }

    async fn get_ip(&self) -> Option<String> {
        let response = self
            .client
            .get("https://api.ipify.org")
            .send()
            .await
            .unwrap();
        if let Ok(body) = response.text().await {
            return Some(body); //once told me
        }
        None
    }

    async fn retrieve_record(&self) -> Option<backends::Record> {
        match &self.config {
            backends::Config::Porkbun { .. } => None,
            backends::Config::Cloudflare {
                subdomain,
                zone_id,
                api_key,
                ..
            } => {
                return backends::cloudflare::retrieve_record(
                    &self.client,
                    &subdomain,
                    &zone_id,
                    &api_key,
                )
                .await;
            }
        }
    }

    async fn update_record(
        &self,
        record: &backends::Record,
        new_ip: &String,
    ) -> Option<backends::Record> {
        match &self.config {
            backends::Config::Porkbun {
                api_key,
                secret_key,
                domain,
                ..
            } => {
                backends::porkbun::update_record(
                    &self.client,
                    domain,
                    api_key,
                    secret_key,
                    record,
                    &new_ip,
                )
                .await
            }
            backends::Config::Cloudflare {
                zone_id,
                api_key,
                domain,
                subdomain,
                ..
            } => {
                backends::cloudflare::update_record(
                    &self.client,
                    &domain,
                    &subdomain,
                    &zone_id,
                    &api_key,
                    record,
                    &new_ip,
                )
                .await
            }
        }
    }
}

fn get_config_dir() -> String {
    format!(
        "{}{}{}{}",
        home::home_dir().unwrap_or_default().display(),
        std::path::MAIN_SEPARATOR_STR,
        ".ddns",
        std::path::MAIN_SEPARATOR_STR
    )
}

#[tokio::main]
async fn main() -> () {
    let arguments = std::env::args().collect::<Vec<String>>();

    println!("ddns-client v{}", env!("CARGO_PKG_VERSION"));

    let first_arg = arguments.get(1);

    let config_dir = get_config_dir();

    let path = match first_arg {
        Some(arg) => arg.clone(),
        None => {
            println!("No custom config path specified, will load from home directory.");
            format!("{}{}", config_dir, "config.json".to_string())
        }
    };

    println!("Using user config file: {}", path);

    let client = DDNSClient::new(&path).await;

    let mut current_record = client.retrieve_record().await.unwrap_or_default();
    let mut current_ip = current_record.content.clone();

    loop {
        println!("Checking IP for change...");
        let new_ip = client.get_ip().await;

        match new_ip {
            Some(ip) => {
                if ip != current_ip {
                    println!("IP has changed from {} to {}", current_ip, ip);

                    let new_record = client.update_record(&current_record, &ip).await;
                    match new_record {
                        Some(record) => {
                            current_record = record;
                            current_ip = current_record.content.clone();
                        }
                        None => println!("Failed to update record. IP has not been changed."),
                    }
                } else {
                    println!("IP has not changed.")
                }
            }
            None => println!("Failed to retrieve IP"),
        }

        sleep(client.timeout).await;
    }
}
