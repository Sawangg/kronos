use chrono::{DateTime, NaiveDateTime};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct OHLCVData {
    pub timestamp: NaiveDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

pub async fn polygon_aggregate(
    symbol: &str,
    multiplier: u64,
    timespan: &str,
    from: &str,
    to: &str,
) -> Result<Vec<OHLCVData>, Box<dyn Error>> {
    let api_key = "iOJZI2Hw1BxdehruJ6p4LvByStwB7UDS";

    let url = format!(
        "https://api.polygon.io/v2/aggs/ticker/{}/range/{}/{}/{}/{}?apiKey={}",
        symbol, multiplier, timespan, from, to, api_key
    );

    let client = Client::new();
    let response = client.get(url).send().await?;
    let body = response.json::<Value>().await?;

    let mut data = Vec::new();
    if let Some(results) = body.get("results").and_then(|r| r.as_array()) {
        for result in results {
            let timestamp = result["t"].as_i64().unwrap();
            let open = result["o"].as_f64().unwrap();
            let high = result["h"].as_f64().unwrap();
            let low = result["l"].as_f64().unwrap();
            let close = result["c"].as_f64().unwrap();
            let volume = result["v"].as_f64().unwrap() as u64;

            let ts = DateTime::from_timestamp(timestamp / 1000, 0).expect("Invalid timestamp");
            data.push(OHLCVData {
                timestamp: ts.naive_utc(),
                open,
                high,
                low,
                close,
                volume,
            });
        }
    }

    Ok(data)
}
