use crate::broker::{broker::Broker, fee::FeeType};
use crate::data::polygon_aggregate;
use crate::engine::{BacktestEngine, BacktestResult};
use axum::{http::StatusCode, Json};
use serde::Deserialize;

use crate::strategy::sma_crossover::SMACrossoverStrategy;

#[derive(Deserialize)]
pub struct CreateSimulation {
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

#[axum::debug_handler]
pub async fn run(
    Json(payload): Json<CreateSimulation>,
) -> (StatusCode, Json<Result<BacktestResult, &'static str>>) {
    let start_date = &payload.parameters.start_date;
    let end_date = &payload.parameters.end_date;

    let strategy = Box::new(SMACrossoverStrategy::new(5, 200));

    let mut engine = BacktestEngine::new(strategy);
    engine.set_time(
        &format!("{} 00:00:00", start_date),
        &format!("{} 00:00:00", end_date),
    );

    // if let Some(tick) = &payload.parameters.tick {
    //     engine.set_tick(tick.parse().unwrap_or_default());
    // }

    // TODO: Bring your own data
    let data_feed = polygon_aggregate(&payload.data, 1, "day", start_date, end_date)
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
        Ok(result) => (StatusCode::OK, Json(Ok(result))),
        Err(error_message) => (StatusCode::INTERNAL_SERVER_ERROR, Json(Err(error_message))),
    }
}
