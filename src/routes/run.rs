use crate::broker::{fee::FeeType, Broker};
use crate::data::polygon_aggregate;
use crate::engine::{BacktestResult, Engine};
use crate::strategy::wasm::WasmStrategy;
use axum::{http::StatusCode, Json};
use chrono::{Duration, NaiveDateTime};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Body {
    parameters: SimulationParameters,
    data: String,
    broker: BrokerSettings,
    strategy: StrategyConfig,
}

#[derive(Deserialize)]
struct StrategyConfig {
    wasm_base64: String,
}

#[derive(Deserialize)]
struct SimulationParameters {
    start_date: String,
    end_date: String,
    tick: Option<String>,
}

#[derive(Deserialize)]
struct BrokerSettings {
    cash: f64,
    fees: Option<FeeType>,
    slippage: Option<SlippageSettings>,
}

#[derive(Deserialize)]
struct SlippageSettings {
    min: f64,
    max: f64,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum Response<T> {
    Success(T),
    Error(&'static str),
}

pub async fn run(Json(payload): Json<Body>) -> (StatusCode, Json<Response<BacktestResult>>) {
    let parse_time = |time_str: &str| -> Result<NaiveDateTime, &'static str> {
        NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid date format")
    };

    let start_date = match parse_time(&payload.parameters.start_date) {
        Ok(date) => date,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(Response::Error(e))),
    };

    let end_date = match parse_time(&payload.parameters.end_date) {
        Ok(date) => date,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(Response::Error(e))),
    };

    let wasm_bytes = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &payload.strategy.wasm_base64,
    ) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error("Invalid base64 encoded WASM")),
            );
        }
    };

    let strategy = match WasmStrategy::new(&wasm_bytes) {
        Ok(s) => Box::new(s),
        Err(e) => {
            eprintln!("Failed to load WASM strategy: {:?}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error("Failed to load WASM strategy")),
            );
        }
    };

    let mut engine = Engine::new(strategy, (start_date, end_date));

    if let Some(tick) = &payload.parameters.tick {
        let duration = match tick.trim_end_matches(['s', 'n']).parse::<i64>() {
            Ok(value) => {
                if tick.ends_with("ns") {
                    Duration::new(0, value as u32)
                } else {
                    Duration::new(value, 0)
                }
            }
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(Response::Error("Cannot parse tick duration")),
                );
            }
        };

        match duration {
            Some(d) => engine.set_tick(d),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(Response::Error("Invalid tick duration value")),
                );
            }
        }
    }

    let data_feed = match polygon_aggregate(
        &payload.data,
        1,
        "day",
        &payload.parameters.start_date[..10],
        &payload.parameters.end_date[..10],
    )
    .await
    {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to fetch OHLCV data: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Response::Error("Failed to fetch OHLCV data")),
            );
        }
    };

    engine.add_data(data_feed);

    let mut broker = Broker::new();
    broker.set_cash(payload.broker.cash);
    if let Some(fees) = payload.broker.fees {
        broker.set_fees(fees);
    }
    if let Some(slippage) = &payload.broker.slippage {
        broker.set_slippage(slippage.min, slippage.max);
    }

    engine.set_broker(broker);

    match engine.run() {
        Ok(result) => (StatusCode::OK, Json(Response::Success(result))),
        Err(error_message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::Error(error_message)),
        ),
    }
}
