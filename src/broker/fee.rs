use serde::Deserialize;

#[derive(Deserialize)]
pub enum FeeType {
    Flat(f64),
    Percentage(f64),
}
