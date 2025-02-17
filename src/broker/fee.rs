// TODO: Allow for one time fees per trade
use serde::Deserialize;

#[derive(Deserialize)]
pub enum FeeType {
    Flat(f64),
    Percentage(f64),
}
