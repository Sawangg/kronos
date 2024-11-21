use std::collections::HashMap;

use crate::data::OHLCVData;

pub struct Position {
    pub quantity: f64,
    pub average_price: f64,
}

impl Position {
    pub fn new(quantity: f64, price: f64) -> Self {
        Position {
            quantity,
            average_price: price,
        }
    }

    pub fn update(&mut self, quantity: f64, price: f64) {
        let total_cost = self.average_price * self.quantity + price * quantity;
        self.quantity += quantity;
        self.average_price = total_cost / self.quantity;
    }

    pub fn remove(&mut self, quantity: f64) -> Result<(), String> {
        if quantity > self.quantity {
            return Err("Cannot sell more than the available quantity.".to_string());
        }
        self.quantity -= quantity;
        Ok(())
    }
}

pub enum FeeType {
    Flat(f64),
    Percentage(f64),
}

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
}

pub struct Broker {
    pub added_funds: f64,
    pub cash: f64,
    pub fee_type: Option<FeeType>,
    pub portfolio: HashMap<String, Position>,
    pub orders: Vec<Order>,
    pub total_placed_orders: i32,
    pub total_exec_orders: i32,
    pub total_fees: f64,
}

impl Broker {
    pub fn new() -> Self {
        Broker {
            added_funds: 0.0,
            cash: 0.0,
            fee_type: None,
            portfolio: HashMap::new(),
            orders: vec![],
            total_placed_orders: 0,
            total_exec_orders: 0,
            total_fees: 0.0,
        }
    }

    pub fn set_cash(&mut self, cash: f64) {
        self.added_funds += cash;
        self.cash = cash;
    }

    pub fn set_fees(&mut self, fee_type: FeeType) {
        self.fee_type = Some(fee_type);
    }

    fn calculate_fees(&mut self, amount: f64) -> f64 {
        match &self.fee_type {
            Some(FeeType::Flat(fee)) => *fee,
            Some(FeeType::Percentage(percentage)) => amount * *percentage,
            _ => 0.0,
        }
    }

    pub fn place_order(&mut self, order: Order) {
        self.total_placed_orders += 1;
        self.orders.push(order);
    }

    pub fn handle_unfulfilled_orders(&mut self, current: &OHLCVData) {
        let mut i = 0;
        while i < self.orders.len() {
            let order = &self.orders[i];

            match order.order_type {
                OrderType::Market => {
                    if let Err(e) = self.execute_order(order.clone(), current.open) {
                        eprintln!("Failed to execute order: {}", e);
                        i += 1;
                    } else {
                        println!(
                            "Executed an order on {} at {}",
                            current.timestamp, current.open
                        );
                        self.total_exec_orders += 1;
                        self.orders.remove(i);
                    }
                }
                OrderType::Limit(_) | OrderType::Stop(_) => {
                    i += 1;
                }
            }
        }
    }

    fn execute_order(&mut self, order: Order, market_price: f64) -> Result<(), String> {
        match order.direction {
            OrderDirection::Buy => {
                let total_cost = order.size * market_price;
                let fees = self.calculate_fees(total_cost);
                let total_spent = total_cost + fees;

                if self.cash >= total_spent {
                    self.cash -= total_spent;
                    self.total_fees += fees;

                    let position = self
                        .portfolio
                        .entry(order.asset.clone())
                        .or_insert_with(|| Position::new(0.0, market_price));

                    position.update(order.size, market_price);
                    Ok(())
                } else {
                    Err("Not enough cash".to_string())
                }
            }
            OrderDirection::Sell => {
                let total_raw_value = order.size * market_price;
                let fees = self.calculate_fees(total_raw_value);
                let total_value = total_raw_value - fees;

                if let Some(position) = self.portfolio.get_mut(&order.asset) {
                    if position.quantity < order.size {
                        Err("Not enough quantity to sell".to_string())
                    } else if let Err(e) = position.remove(order.size) {
                        Err(e)
                    } else {
                        self.cash += total_value;
                        self.total_fees += fees;

                        if position.quantity == 0.0 {
                            self.portfolio.remove(&order.asset);
                        }
                        Ok(())
                    }
                } else {
                    Err("Position not found in portfolio".to_string())
                }
            }
        }
    }

    // Return the total value of all the positions on the close of the tick
    pub fn portfolio_value(&self, data: &OHLCVData) -> f64 {
        let mut total_value = 0.0;

        for (asset, position) in &self.portfolio {
            let current_price = data.close;
            total_value += position.quantity * current_price;
            println!(
                "Asset: {}, Quantity: {}, Price: {}",
                asset, position.quantity, current_price
            );
        }

        total_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn create_dummy_price(open: f64, high: f64, low: f64, close: f64) -> OHLCVData {
        OHLCVData {
            timestamp: NaiveDateTime::parse_from_str("1999-11-01 00:00:00", "%Y-%m-%d %H:%M:%S")
                .expect("Invalid date"),
            open,
            high,
            low,
            close,
            volume: 1000,
        }
    }

    #[test]
    fn is_order_placed() {
        let mut broker = Broker::new();
        let order = Order {
            asset: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            size: 1.0,
            order_type: OrderType::Market,
        };
        broker.place_order(order);

        assert_eq!(broker.total_placed_orders, 1);
        assert_eq!(broker.total_exec_orders, 0);
        assert_eq!(broker.orders.len(), 1);
        assert_eq!(broker.orders[0].asset, "AAPL");
        assert_eq!(broker.orders[0].direction, OrderDirection::Buy);
        assert_eq!(broker.orders[0].size, 1.0);
        assert_eq!(broker.orders[0].order_type, OrderType::Market);
    }

    #[test]
    fn is_buy_market_order_executed() {
        let mut broker = Broker::new();
        let order = Order {
            asset: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            size: 1.0,
            order_type: OrderType::Market,
        };
        broker.set_cash(1000.0);
        broker.set_fees(FeeType::Flat(1.0));
        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(100.0, 101.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&dummy_price);

        // Check the cash in our balance after the execution (order price + fees)
        assert_eq!(broker.cash, 899.0);
        assert_eq!(broker.portfolio_value(&dummy_price), 99.0);

        // Check if the asset is in the portfolio
        let position = broker.portfolio.get("AAPL").unwrap();
        assert_eq!(position.quantity, 1.0);
        assert_eq!(position.average_price, 100.0);
    }

    #[test]
    fn not_enough_cash() {
        let mut broker = Broker::new();
        let order = Order {
            asset: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            size: 1.0,
            order_type: OrderType::Market,
        };
        broker.set_fees(FeeType::Flat(1.0));
        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(100.0, 101.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&dummy_price);

        // Check the cash in our balance after the execution (order price + fees)
        assert_eq!(broker.cash, 0.0);
        assert_eq!(broker.portfolio_value(&dummy_price), 0.0);

        // Check if there is no assets in the portolio
        assert!(!broker.portfolio.contains_key("AAPL"));
    }

    #[test]
    fn add_to_existing_position() {
        let mut broker = Broker::new();
        let order = Order {
            asset: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            size: 1.0,
            order_type: OrderType::Market,
        };
        broker.set_cash(1000.0);
        broker.set_fees(FeeType::Flat(1.0));
        broker
            .portfolio
            .insert("AAPL".to_string(), Position::new(1.0, 100.0));

        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(110.0, 111.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&dummy_price);

        // Check if the assets are in the portfolio
        let position = broker.portfolio.get("AAPL").unwrap();
        assert_eq!(position.quantity, 2.0);

        // Calculate the new average price: (100 * 1 + 110 * 1) / 2 = 105
        assert_eq!(position.average_price, 105.0);
    }

    #[test]
    fn is_sell_market_order_executed() {
        let mut broker = Broker::new();
        let order = Order {
            asset: "AAPL".to_string(),
            direction: OrderDirection::Sell,
            size: 1.0,
            order_type: OrderType::Market,
        };
        broker.set_cash(1000.0);
        broker.set_fees(FeeType::Flat(1.0));
        broker
            .portfolio
            .insert("AAPL".to_string(), Position::new(1.0, 100.0));

        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(100.0, 101.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&dummy_price);

        // Check the cash after execution (1000 + 100 - 1 (cash + position - fee))
        assert_eq!(broker.cash, 1099.0);

        assert_eq!(broker.portfolio.len(), 0);
    }
}
