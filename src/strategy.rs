use crate::{broker::broker::Broker, data::OHLCVData};
use chrono::NaiveDateTime;

pub mod wasm;

pub trait Strategy {
    fn init(&mut self);
    fn tick(&mut self, current_time: &NaiveDateTime, data: Option<&OHLCVData>, broker: &mut Broker);
}
