use flexi_logger::Logger;
use log::{error, info, warn};
use reqwest::Client;
use tokio::time::{sleep, Duration};

pub mod backends;

struct DDNSClient {
    client: Client,
    config: backends::Config,
    timeout: Duration,
}

impl DDNSClient {
    async fn new(config_path: &str) -> Self {
        let client = Client::new();
        let file = std::fs::read_to_string(config_path).unwrap_or_else(|err| {
            error!("Error reading file: {}", err);
            std::process::exit(1);
        });

        let config: backends::Config = serde_json::from_str(&file).unwrap();

        let timeout = Duration::from_secs(config.update_every_seconds);

        Self {
            client,
            config,
            timeout,
        }
    }

    async fn get_ip(&self) -> Option<String> {
        let response = self.client.get("https://api.ipify.org").send().await.ok()?;

        if let Ok(body) = response.text().await {
            return Some(body); //once told me
        }
        None
    }

    async fn retrieve_record(&self) -> Option<backends::Record> {
        match &self.config.backend {
            backends::BackendConfig::Porkbun { .. } => None,
            backends::BackendConfig::Cloudflare {
                subdomain,
                zone_id,
                api_key,
                ..
            } => {
                return backends::cloudflare::retrieve_record(
                    &self.client,
                    subdomain,
                    zone_id,
                    api_key,
                )
                .await;
            }
        }
    }

    async fn update_record(
        &self,
        record: &backends::Record,
        new_ip: &str,
    ) -> Option<backends::Record> {
        match &self.config.backend {
            backends::BackendConfig::Porkbun {
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
                    new_ip,
                )
                .await
            }
            backends::BackendConfig::Cloudflare {
                zone_id,
                api_key,
                domain,
                subdomain,
                ..
            } => {
                backends::cloudflare::update_record(
                    &self.client,
                    domain,
                    subdomain,
                    zone_id,
                    api_key,
                    record,
                    new_ip,
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
async fn main() {
    let arguments = std::env::args().collect::<Vec<String>>();

    Logger::try_with_str("info")
        .unwrap()
        .log_to_stdout()
        .format(flexi_logger::detailed_format)
        .start()
        .unwrap();

    info!("ddns-client v{}", env!("CARGO_PKG_VERSION"));

    let first_arg = arguments.get(1);

    let config_dir = get_config_dir();

    let path = match first_arg {
        Some(arg) => arg.clone(),
        None => {
            warn!("No custom config path specified, will load from home directory.");
            format!("{}config.json", config_dir)
        }
    };

    info!("Using user config file: {}", path);

    let client = DDNSClient::new(&path).await;

    let mut current_record = client.retrieve_record().await.unwrap_or_default();
    let mut current_ip = current_record.content.clone();

    loop {
        info!("Checking IP for change...");
        let new_ip = client.get_ip().await;

        match new_ip {
            Some(ip) => {
                if ip != current_ip {
                    info!("IP has changed from {} to {}", current_ip, ip);

                    let new_record = client.update_record(&current_record, &ip).await;
                    match new_record {
                        Some(record) => {
                            current_record = record;
                            current_ip = current_record.content.clone();
                        }
                        None => error!("Failed to update record. IP has not been changed."),
                    }
                } else {
                    info!("IP has not changed.")
                }
            }
            None => error!("Failed to retrieve IP"),
        }

        sleep(client.timeout).await;
    }
}
