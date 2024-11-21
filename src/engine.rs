use crate::broker::Broker;
use crate::data::OHLCVData;
use crate::strategy::Strategy;
use colored::Colorize;

pub struct BacktestEngine {
    pub broker: Broker,
    pub data_feed: Vec<OHLCVData>,
    pub strategy: Box<dyn Strategy>,
}

impl BacktestEngine {
    pub fn new(strategy: Box<dyn Strategy>) -> Self {
        BacktestEngine {
            broker: Broker::new(),
            data_feed: vec![],
            strategy,
        }
    }

    pub fn set_broker(&mut self, broker: Broker) {
        self.broker = broker;
    }

    pub fn add_data(&mut self, data: Vec<OHLCVData>) {
        self.data_feed = data;
    }

    pub fn run(&mut self) {
        self.strategy.init();

        for i in 0..self.data_feed.len() {
            self.broker.handle_unfulfilled_orders(&self.data_feed[i]); // Execute orders on the next tick

            let data = &self.data_feed[0..=i];
            self.strategy.next(data, &mut self.broker);

            self.strategy.log();
        }

        let last_day = self.data_feed.last().expect("No data found");
        let profit = f64::trunc(
            ((self.broker.cash + self.broker.portfolio_value(&last_day)) - self.broker.added_funds)
                * 100.0,
        ) / 100.0;

        // Simulation ended print the report
        println!(
            "===============================\nCash: {}\nPortfolio value: {}\nProfit: {} ({}%)\nNumber of order placed: {}\nNumber of order executed: {}\nTotal commissions: {}\n===============================",
            f64::trunc(self.broker.cash * 100.0) / 100.0,
            f64::trunc(self.broker.portfolio_value(&last_day) * 100.0) / 100.0,
            if profit >= 0.0 { profit.to_string().green() } else { profit.to_string().red() },
            f64::trunc(((profit / self.broker.added_funds) * 100.0) * 100.0) / 100.0,
            self.broker.total_placed_orders,
            self.broker.total_exec_orders,
            self.broker.total_fees,
        );
    }
}
