mod broker;
mod data;
mod engine;
mod strategy;

use crate::broker::Broker;
use crate::broker::FeeType;
use crate::engine::BacktestEngine;
use crate::strategy::sma_crossover::SMACrossoverStrategy;

#[tokio::main]
async fn main() {
    let strategy = Box::new(SMACrossoverStrategy::new(5, 10));

    let mut engine = BacktestEngine::new(strategy);

    //let data_feed = data::alphavantage_day("AAPL")
    //    .await
    //    .expect("Failed to fetch OHLCV data for symbol");

    let data_feed = data::polygon_aggregate("AAPL", 1, "day", "2022-11-19", "2024-11-19")
        .await
        .expect("Failed to fetch OHLCV data for symbol");

    engine.add_data(data_feed);

    let mut broker = Broker::new();
    broker.set_cash(10_000.0);
    broker.set_fees(FeeType::Flat(1.0));

    engine.set_broker(broker);

    engine.run();
}
