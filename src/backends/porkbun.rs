use crate::Record;
use reqwest::Client;
use serde_json::json;

const API_ENDPOINT: &str = "https://porkbun.com/api/json/v3/dns/";

pub async fn retrieve_record(
    client: &Client,
    domain: &str,
    subdomain: &str,
    api_key: &str,
    secret_key: &str,
) -> Option<Record> {
    let url = format!("{}/retrieve/{}", API_ENDPOINT, domain);

    let response = client.post(url).json(&json!({
        "apikey":api_key,
        "secretapikey":secret_key
    }));

    let json = response
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    if json["status"].as_str().unwrap() != "SUCCESS" {
        return None;
    }

    let records = json["records"].as_array().unwrap();

    match records.iter().find(|record| {
        record["name"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{}.", subdomain))
    }) {
        Some(record) => Some(Record {
            id: record["id"].as_str().unwrap().to_string(),
            name: record["name"].as_str().unwrap().to_string(),
            record_type: record["type"].as_str().unwrap().to_string(),
            content: record["content"].as_str().unwrap().to_string(),
        }),
        None => None,
    }
}

async fn retrieve_record_with_id(
    client: &Client,
    domain: &str,
    record_id: &str,
    api_key: &str,
    secret_key: &str,
) -> Option<Record> {
    let url = format!("{}/retrieve/{}/{}", API_ENDPOINT, domain, record_id);

    let response = client.post(url).json(&json!({
        "apikey": api_key,
        "secretapikey": secret_key,
    }));

    let json = response
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    if json["status"].as_str().unwrap() != "SUCCESS" {
        return None;
    }

    let record = json["record"].as_array().unwrap().get(0);

    match record {
        None => None,
        Some(record) => Some(Record {
            id: record["id"].as_str().unwrap().to_string(),
            name: record["name"].as_str().unwrap().to_string(),
            record_type: record["type"].as_str().unwrap().to_string(),
            content: record["content"].as_str().unwrap().to_string(),
        }),
    }
}

pub async fn update_record(
    client: &Client,
    domain: &str,
    api_key: &str,
    secret_key: &str,
    record: &Record,
    new_ip: &str,
) -> Option<Record> {
    let url = format!("{}/edit/{}/{}", API_ENDPOINT, domain, record.id);

    let response = client.post(url).json(&json!({
        "apikey": api_key,
        "secretapikey": secret_key,
        "content": new_ip,
        "type": record.record_type
    }));

    let json = response
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    if json["status"].as_str().unwrap() != "SUCCESS" {
        return None;
    }

    if let Some(updated_record) =
        retrieve_record_with_id(client, domain, &record.id, api_key, secret_key).await
    {
        return Some(updated_record);
    }

    None
}
