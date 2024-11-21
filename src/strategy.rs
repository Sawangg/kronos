use crate::{broker::Broker, data::OHLCVData};

pub mod sma_crossover;

pub trait Strategy {
    fn init(&mut self);
    fn next(&mut self, data: &[OHLCVData], broker: &mut Broker);
    fn log(&self);
}
