use crate::configuration::Record;
use reqwest::Client;

const API_ENDPOINT: &str = "https://api.cloudflare.com/client/v4";

pub async fn retrieve_record(
    client: &Client,
    domain: &str,
    zone_id: &str,
    api_key: &str,
) -> Option<Record> {
    let url = format!("{}/zones/{}/dns_records", API_ENDPOINT, zone_id);

    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key));
    let json = response
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    if !json["success"].as_bool().unwrap() {
        return None;
    }

    let records = json["result"].as_array().unwrap();

    match records
        .iter()
        .find(|record| record["name"].as_str().unwrap() == domain)
    {
        Some(record) => Some(Record {
            id: record["id"].as_str().unwrap().to_string(),
            name: record["name"].as_str().unwrap().to_string(),
            record_type: record["type"].as_str().unwrap().to_string(),
            content: record["content"].as_str().unwrap().to_string(),
        }),
        None => None,
    }
}
