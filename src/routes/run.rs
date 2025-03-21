use crate::broker::{broker::Broker, fee::FeeType};
use crate::data::polygon_aggregate;
use crate::engine::{BacktestResult, Engine};
use axum::{http::StatusCode, Json};
use chrono::{Duration, NaiveDateTime};
use serde::Deserialize;

use crate::strategy::sma_crossover::SMACrossoverStrategy;

#[derive(Deserialize)]
pub struct Body {
    parameters: SimulationParameters,
    data: String,
    broker: BrokerSettings,
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
    let parse_time = |time_str: &str| {
        NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid date format")
    };

    let start_date = parse_time(&payload.parameters.start_date).expect("Invalid start_date format");
    let end_date = parse_time(&payload.parameters.end_date).expect("Invalid end_date format");

    // TODO: remove this when WASM based strategies are implemented
    let strategy = Box::new(SMACrossoverStrategy::new(5, 200));

    let mut engine = Engine::new(strategy, (start_date, end_date));

    if let Some(tick) = &payload.parameters.tick {
        let duration = if let Ok(value) = tick
            .trim_end_matches(|c| c == 's' || c == 'n')
            .parse::<i64>()
        {
            if tick.ends_with("ns") {
                Duration::new(0, value as u32)
            } else {
                Duration::new(value, 0)
            }
        } else {
            return (
                StatusCode::BAD_REQUEST,
                Json(Response::Error("Cannot parse tick duration")),
            );
        };

        engine.set_tick(duration.expect("Cannot parse tick duration"));
    }

    // TODO: Bring your own data
    let data_feed = polygon_aggregate(
        &payload.data,
        1,
        "day",
        &payload.parameters.start_date[..10],
        &payload.parameters.end_date[..10],
    )
    .await
    .expect("Failed to fetch OHLCV data");

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
