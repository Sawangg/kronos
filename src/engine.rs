use crate::broker::broker::Broker;
use crate::data::OHLCVData;
use crate::strategy::Strategy;
use chrono::{Duration, NaiveDateTime};
use serde::Serialize;

#[derive(Serialize)]
pub struct BacktestResult {
    pub cash: f64,
    pub portfolio_value: f64,
    pub profit: f64,
    pub profit_percentage: f64,
    pub num_orders_placed: i32,
    pub num_orders_executed: i32,
    pub total_fees: f64,
    pub total_slippage: f64,
}

pub struct BacktestEngine {
    pub broker: Broker,
    pub data_feed: Vec<OHLCVData>,
    pub strategy: Box<dyn Strategy + Send>,
    pub time_range: Option<(NaiveDateTime, NaiveDateTime)>,
    pub tick: Duration,
}

impl BacktestEngine {
    pub fn new(strategy: Box<dyn Strategy + Send>) -> Self {
        BacktestEngine {
            broker: Broker::new(),
            data_feed: vec![],
            strategy,
            time_range: None,
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

    pub fn set_time(&mut self, from: &str, to: &str) {
        let parse_time = |time_str: &str| {
            NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|_| "Invalid date format")
        };

        let from_time = parse_time(from).expect("Invalid 'from' date format");
        let to_time = parse_time(to).expect("Invalid 'to' date format");

        self.time_range = Some((from_time, to_time));
    }

    pub fn set_tick(&mut self, tick: Duration) {
        self.tick = tick;
    }

    // TODO: use unix timestamp instead of chrono for efficiency
    pub fn run(&mut self) -> Result<BacktestResult, &'static str> {
        self.strategy.init();

        if self.data_feed.is_empty() {
            return Err("Error: Data feed is empty.");
        }

        let (from_time, to_time) = self.time_range.as_ref().expect("Time range is not set");

        let mut current_time = *from_time;
        let mut data_index = 0;

        while current_time <= *to_time {
            if data_index + 1 < self.data_feed.len() {
                let next_data = &self.data_feed[data_index + 1];
                if next_data.timestamp <= current_time {
                    data_index += 1;
                }
            }

            if let Some(current_price) = self.data_feed.get(data_index) {
                self.broker
                    .handle_unfulfilled_orders(&current_time, current_price);

                let previous_data = &self.data_feed[..=data_index];

                self.strategy
                    .tick(&current_time, previous_data, &mut self.broker);
            } else {
                println!("No data available for the current time: {}", current_time);
            }

            current_time += self.tick;

            // Exit if we've gone past the last data timestamp
            if current_time > self.data_feed.last().unwrap().timestamp {
                break;
            }
        }

        // Creating feedback data
        let last_tick = self.data_feed.last().expect("No data found");
        let profit = f64::trunc(
            ((self.broker.cash + self.broker.portfolio_value(&last_tick))
                - self.broker.added_funds)
                * 100.0,
        ) / 100.0;

        let profit_percentage =
            f64::trunc(((profit / self.broker.added_funds) * 100.0) * 100.0) / 100.0;

        Ok(BacktestResult {
            cash: f64::trunc(self.broker.cash * 100.0) / 100.0,
            portfolio_value: f64::trunc(self.broker.portfolio_value(&last_tick) * 100.0) / 100.0,
            profit,
            profit_percentage,
            num_orders_placed: self.broker.total_placed_orders,
            num_orders_executed: self.broker.total_exec_orders,
            total_fees: self.broker.total_fees,
            total_slippage: self.broker.total_slippage,
        })
    }
}
