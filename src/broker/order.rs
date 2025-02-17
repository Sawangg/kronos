use chrono::NaiveDateTime;

#[derive(Debug, Clone, PartialEq)]
pub enum OrderType {
    Market,
    Limit(f64),
    Stop(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Buy,
    Sell,
}

#[derive(Clone)]
pub struct Order {
    pub asset: String,
    pub direction: OrderDirection,
    pub size: f64,
    pub order_type: OrderType,
    pub valid_until: Option<NaiveDateTime>,
}
