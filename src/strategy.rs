use crate::{broker::broker::Broker, data::OHLCVData};
use chrono::NaiveDateTime;

pub mod sma_crossover;

pub trait Strategy {
    fn init(&mut self);
    fn tick(&mut self, current_time: &NaiveDateTime, data: &[OHLCVData], broker: &mut Broker);
}
