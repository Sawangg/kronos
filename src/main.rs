mod broker;
mod data;
mod engine;
mod strategy;

use chrono::Duration;

use crate::broker::Broker;
use crate::broker::FeeType;
use crate::engine::BacktestEngine;
use crate::strategy::sma_crossover::SMACrossoverStrategy;

#[tokio::main]
async fn main() {
    let strategy = Box::new(SMACrossoverStrategy::new(5, 200));

    let start_date = "2022-11-23";
    let end_date = "2024-11-23";

    let mut engine = BacktestEngine::new(strategy);
    engine.set_time(
        &format!("{} 00:00:00", start_date),
        &format!("{} 00:00:00", end_date),
    );
    //engine.set_tick(Duration::seconds(1));

    //let data_feed = data::alphavantage_day("AAPL")
    //.await
    //.expect("Failed to fetch OHLCV data");

    let data_feed = data::polygon_aggregate("AAPL", 1, "day", start_date, end_date)
        .await
        .expect("Failed to fetch OHLCV data");
    engine.add_data(data_feed);

    let mut broker = Broker::new();
    broker.set_cash(10_000.0);
    broker.set_fees(FeeType::Flat(1.0));
    broker.set_slippage(-0.005, 0.005); // Set slippage between -0.5% and +0.5%

    engine.set_broker(broker);

    engine.run();
}
