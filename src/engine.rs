use crate::broker::Broker;
use crate::data::OHLCVData;
use crate::strategy::Strategy;
use chrono::{Duration, NaiveDateTime};
use colored::Colorize;

pub struct BacktestEngine {
    pub broker: Broker,
    pub data_feed: Vec<OHLCVData>,
    pub strategy: Box<dyn Strategy>,
    pub slippage: (Duration, Option<Duration>),
    pub time_range: Option<(NaiveDateTime, NaiveDateTime)>,
    pub tick: Duration,
}

impl BacktestEngine {
    pub fn new(strategy: Box<dyn Strategy>) -> Self {
        BacktestEngine {
            broker: Broker::new(),
            data_feed: vec![],
            strategy,
            slippage: (Duration::minutes(1), None),
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

    pub fn set_slipage(&mut self, slipage: Duration, max_slipage: Option<Duration>) {
        self.slippage = (slipage, max_slipage);
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

    pub fn run(&mut self) {
        self.strategy.init();

        if self.data_feed.is_empty() {
            println!("Error: Data feed is empty.");
            return;
        }

        let (from_time, to_time) = self.time_range.as_ref().expect("Time range is not set");

        let mut current_time = *from_time;
        let mut data_index = 0;

        while current_time <= *to_time {
            let mut closest_data: Option<&OHLCVData> = None;
            let mut closest_time_diff = std::i64::MAX;

            while data_index < self.data_feed.len() {
                let ohlcv = &self.data_feed[data_index];
                let time_diff = (ohlcv
                    .timestamp
                    .signed_duration_since(current_time)
                    .num_seconds())
                .abs();

                if time_diff < closest_time_diff {
                    closest_data = Some(ohlcv);
                    closest_time_diff = time_diff;
                }

                if ohlcv.timestamp > current_time {
                    break;
                }

                data_index += 1;
            }

            if let Some(current_price) = closest_data {
                self.broker.handle_unfulfilled_orders(current_price);

                let previous_data = if data_index < self.data_feed.len() {
                    &self.data_feed[..=data_index]
                } else {
                    &self.data_feed[..]
                };

                self.strategy
                    .next(&current_time, previous_data, &mut self.broker);
            } else {
                println!("No data available for the current time: {}", current_time);
            }

            current_time += self.tick;

            // Quit the simulation early if there is no data left
            if current_time > self.data_feed.last().unwrap().timestamp {
                break;
            }
        }

        self.feedback();
    }

    fn feedback(&self) {
        let last_tick = self.data_feed.last().expect("No data found");
        let profit = f64::trunc(
            ((self.broker.cash + self.broker.portfolio_value(&last_tick))
                - self.broker.added_funds)
                * 100.0,
        ) / 100.0;

        println!(
            "===============================\nCash: {}\nPortfolio value: {}\nProfit: {} ({}%)\nNumber of orders placed: {}\nNumber of orders executed: {}\nTotal commissions: {}\n===============================",
            f64::trunc(self.broker.cash * 100.0) / 100.0,
            f64::trunc(self.broker.portfolio_value(&last_tick) * 100.0) / 100.0,
            if profit >= 0.0 { profit.to_string().green() } else { profit.to_string().red() },
            f64::trunc(((profit / self.broker.added_funds) * 100.0) * 100.0) / 100.0,
            self.broker.total_placed_orders,
            self.broker.total_exec_orders,
            self.broker.total_fees,
        );
    }
}
