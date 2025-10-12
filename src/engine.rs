use crate::analytics::{metrics::GlobalMetrics, trade::Trade};
use crate::broker::Broker;
use crate::data::OHLCVData;
use crate::strategy::Strategy;
use chrono::{Duration, NaiveDateTime};
use serde::Serialize;

#[derive(Serialize)]
pub struct BacktestResult {
    pub trades: Vec<Trade>,
    pub metrics: GlobalMetrics,
}

pub struct Engine {
    pub broker: Broker,
    pub data_feed: Vec<OHLCVData>,
    pub strategy: Box<dyn Strategy + Send>,
    pub time_range: (NaiveDateTime, NaiveDateTime),
    pub tick: Duration,
}

impl Engine {
    pub fn new(
        strategy: Box<dyn Strategy + Send>,
        time_range: (NaiveDateTime, NaiveDateTime),
    ) -> Self {
        Engine {
            broker: Broker::new(),
            data_feed: vec![],
            strategy,
            time_range,
            tick: Duration::minutes(1),
        }
    }

    pub fn add_data(&mut self, data: Vec<OHLCVData>) {
        // TODO: sort the data by timestamp (oldest to newest)
        self.data_feed = data;
    }

    pub fn set_broker(&mut self, broker: Broker) {
        self.broker = broker;
    }

    pub fn set_tick(&mut self, tick: Duration) {
        self.tick = tick;
    }

    // TODO: cut loop time by optimizing time with trading days for equities (45% time decrease)
    pub fn run(&mut self) -> Result<BacktestResult, &'static str> {
        let timer = std::time::Instant::now();

        self.strategy.init();

        if self.data_feed.is_empty() {
            return Err("Error: Data feed is empty.");
        }

        let (start_time, end_time) = self.time_range;

        let mut current_timestamp = start_time.and_utc().timestamp();
        let end_timestamp = end_time.and_utc().timestamp();
        let tick_seconds = self.tick.num_seconds();
        let last_data_timestamp = self
            .data_feed
            .last()
            .unwrap()
            .timestamp
            .and_utc()
            .timestamp();
        let mut data_index = 0;

        while current_timestamp <= end_timestamp {
            let current_time = chrono::DateTime::from_timestamp(current_timestamp, 0)
                .expect("Invalid timestamp")
                .naive_utc();

            if data_index + 1 < self.data_feed.len() {
                let next_data = &self.data_feed[data_index + 1];
                if next_data.timestamp.and_utc().timestamp() <= current_timestamp {
                    data_index += 1;
                }
            }

            if let Some(current_price) = self.data_feed.get(data_index) {
                self.broker
                    .handle_unfulfilled_orders(&current_time, current_price);

                let total_equity = self.broker.cash + self.broker.portfolio_value(current_price);
                self.broker
                    .trade_tracker
                    .record_equity_snapshot(current_time, total_equity);
            }

            let current_candle = self.data_feed.get(data_index);
            self.strategy
                .tick(&current_time, current_candle, &mut self.broker);

            current_timestamp += tick_seconds;

            if current_timestamp > last_data_timestamp {
                break;
            }
        }

        println!("Backtest completed in: {:?}", timer.elapsed());

        let last_tick = self.data_feed.last().expect("No data found");
        let tracker = &self.broker.trade_tracker;

        let closed_trades: Vec<Trade> = tracker.get_closed_trades().to_vec();
        let equity_curve = tracker.get_equity_curve();

        let cash = self.broker.cash;
        let portfolio_value = self.broker.portfolio_value(last_tick);

        let metrics = GlobalMetrics::calculate(
            &closed_trades,
            equity_curve,
            tracker.initial_capital,
            0.03,
            cash,
            portfolio_value,
            self.broker.analytics.total_placed_orders,
            self.broker.analytics.total_exec_orders,
            tracker.total_fees,
            tracker.total_slippage,
        );

        Ok(BacktestResult {
            trades: closed_trades,
            metrics,
        })
    }
}
