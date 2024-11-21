use crate::broker::{Broker, Order, OrderDirection, OrderType};
use crate::data::OHLCVData;
use crate::strategy::Strategy;

pub struct SMACrossoverStrategy {
    short_period: usize,
    long_period: usize,
    short_sma: f64,
    long_sma: f64,
    position_open: bool,
}

impl SMACrossoverStrategy {
    pub fn new(short_period: usize, long_period: usize) -> Self {
        SMACrossoverStrategy {
            short_period,
            long_period,
            short_sma: 0.0,
            long_sma: 0.0,
            position_open: false,
        }
    }

    fn calculate_sma(data: &[OHLCVData], period: usize) -> f64 {
        if data.len() < period {
            return 0.0;
        }
        let sum: f64 = data[data.len() - period..]
            .iter()
            .map(|x| x.close) // We assume you want to use the close price for SMA
            .sum();
        sum / period as f64
    }

    fn check_for_crossover(&mut self, data: &[OHLCVData], broker: &mut Broker) {
        let current_short_sma = SMACrossoverStrategy::calculate_sma(&data, self.short_period);
        let current_long_sma = SMACrossoverStrategy::calculate_sma(&data, self.long_period);

        // Buy Signal: Short SMA crosses above Long SMA
        if !self.position_open && current_short_sma > current_long_sma {
            let order = Order {
                asset: "AAPL".to_string(),
                direction: OrderDirection::Buy,
                order_type: OrderType::Market,
                size: 1.0,
            };
            broker.place_order(order);
            //println!("Buy signal: Short SMA crossed above Long SMA");
            self.position_open = true;
        }

        // Sell Signal: Short SMA crosses below Long SMA
        if self.position_open && current_short_sma < current_long_sma {
            let order = Order {
                asset: "AAPL".to_string(),
                direction: OrderDirection::Sell,
                order_type: OrderType::Market,
                size: 1.0,
            };
            broker.place_order(order);
            //println!("Sell signal: Short SMA crossed below Long SMA");
            self.position_open = false;
        }
    }
}

impl Strategy for SMACrossoverStrategy {
    fn init(&mut self) {
        self.position_open = false;
        println!("Initialized SMA Crossover Strategy");
    }

    fn next(&mut self, data: &[OHLCVData], broker: &mut Broker) {
        self.check_for_crossover(data, broker);
    }

    fn log(&self) {}
}
