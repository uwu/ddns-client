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

    async fn get_ip(&self, url: &str) -> Option<String> {
        let response = self.client.get(url).send().await.ok()?;

        if let Ok(body) = response.text().await {
            return Some(body); //once told me
        }
        None
    }

    async fn retrieve_record(&self, record_type: &str) -> Option<backends::Record> {
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
                    record_type,
                )
                .await;
            }
        }
    }

    async fn check_and_update(&self, url: &str, record_type: &str, state: &mut IpState) {
        info!("Checking {} IP for change...", record_type);
        match self.get_ip(url).await {
            Some(ip) if ip != state.ip => {
                info!("IP has changed from {} to {}", state.ip, ip);
                match self.update_record(&state.record, &ip).await {
                    Some(record) => {
                        state.ip = record.content.clone();
                        state.record = record;
                    }
                    None => error!("Failed to update record. IP has not been changed."),
                }
            }
            Some(_) => info!("IP has not changed."),
            None => error!("Failed to retrieve IP"),
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

struct IpState {
    record: backends::Record,
    ip: String,
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

    let mut v4_state = match client.retrieve_record("A").await {
        Some(record) => Some(IpState {
            ip: record.content.clone(),
            record,
        }),
        None => {
            warn!("No A record found, skipping IPv4 updates.");
            None
        }
    };

    let mut v6_state = if client.config.disable_ipv6 {
        info!("IPv6 disabled via config.");
        None
    } else {
        match client.retrieve_record("AAAA").await {
            Some(record) => Some(IpState {
                ip: record.content.clone(),
                record,
            }),
            None => {
                warn!("No AAAA record found, skipping IPv6 updates.");
                None
            }
        }
    };

    loop {
        if let Some(state) = v4_state.as_mut() {
            client
                .check_and_update("https://api4.ipify.org", "A", state)
                .await;
        }

        if let Some(state) = v6_state.as_mut() {
            client
                .check_and_update("https://api6.ipify.org", "AAAA", state)
                .await;
        }

        sleep(client.timeout).await;
    }
}
