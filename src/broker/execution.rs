use crate::broker::{
    fee::FeeType,
    order::{Order, OrderDirection, OrderType},
    position::Position,
};
use crate::data::OHLCVData;
use chrono::NaiveDateTime;
use rand::Rng;
use std::collections::HashMap;

pub struct BrokerAnalytics {
    pub added_funds: f64,
    pub total_placed_orders: i32,
    pub total_exec_orders: i32,
    pub total_fees: f64,
    pub total_slippage: f64,
}

impl BrokerAnalytics {
    pub fn new() -> Self {
        BrokerAnalytics {
            added_funds: 0.0,
            total_placed_orders: 0,
            total_exec_orders: 0,
            total_fees: 0.0,
            total_slippage: 0.0,
        }
    }
}

pub struct Broker {
    pub cash: f64,
    pub fee_type: Option<FeeType>,
    pub slippage_range: (f64, f64),
    pub portfolio: HashMap<String, Position>,
    pub orders: Vec<Order>,
    slippage_values: Vec<f64>,
    slippage_index: usize,
    pub analytics: BrokerAnalytics,
}

impl Broker {
    pub fn new() -> Self {
        Broker {
            cash: 0.0,
            fee_type: None,
            slippage_range: (0.0, 0.0),
            portfolio: HashMap::new(),
            orders: vec![],
            slippage_values: vec![],
            slippage_index: 0,
            analytics: BrokerAnalytics::new(),
        }
    }

    pub fn set_cash(&mut self, cash: f64) {
        self.analytics.added_funds += cash;
        self.cash = cash;
    }

    pub fn set_fees(&mut self, fee_type: FeeType) {
        self.fee_type = Some(fee_type);
    }

    pub fn set_slippage(&mut self, min_slippage: f64, max_slippage: f64) {
        self.slippage_range = (min_slippage, max_slippage);

        self.slippage_values = Vec::with_capacity(10000);
        let mut rng = rand::rng();
        for _ in 0..10000 {
            self.slippage_values
                .push(rng.random_range(min_slippage..=max_slippage));
        }
        self.slippage_index = 0;
    }

    pub fn place_order(&mut self, order: Order) {
        self.analytics.total_placed_orders += 1;
        self.orders.push(order);
    }

    #[inline]
    fn calculate_fees(&mut self, amount: f64) -> f64 {
        match &self.fee_type {
            Some(FeeType::Flat(fee)) => *fee,
            Some(FeeType::Percentage(percentage)) => amount * *percentage,
            _ => 0.0,
        }
    }

    #[inline]
    fn try_execute_and_remove(&mut self, i: &mut usize, order: &Order, price: f64) {
        match self.execute_order(order.clone(), price) {
            Ok(_) => {
                self.analytics.total_exec_orders += 1;
                self.orders.swap_remove(*i);
            }
            Err(e) => {
                eprintln!("Failed to execute order: {}", e);
                *i += 1;
            }
        }
    }

    #[inline]
    pub fn handle_unfulfilled_orders(
        &mut self,
        current_time: &NaiveDateTime,
        current_price: &OHLCVData,
    ) {
        let mut i = 0;
        while i < self.orders.len() {
            let order = self.orders[i].clone();

            if let Some(valid_until) = order.valid_until {
                if current_time > &valid_until {
                    self.orders.swap_remove(i);
                    continue;
                }
            }

            match order.order_type {
                OrderType::Market => {
                    self.try_execute_and_remove(&mut i, &order, current_price.open);
                }
                OrderType::Limit(price) => {
                    if (order.direction == OrderDirection::Buy && current_price.open <= price)
                        || (order.direction == OrderDirection::Sell && current_price.open >= price)
                    {
                        self.try_execute_and_remove(&mut i, &order, current_price.open);
                    } else {
                        i += 1;
                    }
                }
                OrderType::Stop(price) => {
                    if (order.direction == OrderDirection::Buy && current_price.open >= price)
                        || (order.direction == OrderDirection::Sell && current_price.open <= price)
                    {
                        self.try_execute_and_remove(&mut i, &order, current_price.open);
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }

    #[inline]
    fn apply_slippage(&mut self, market_price: f64) -> f64 {
        if self.slippage_values.is_empty() {
            return market_price;
        }

        let slippage_percentage =
            self.slippage_values[self.slippage_index % self.slippage_values.len()];
        self.slippage_index += 1;
        market_price * (1.0 + slippage_percentage)
    }

    fn execute_order(&mut self, order: Order, market_price: f64) -> Result<(), String> {
        let execution_price = self.apply_slippage(market_price);
        let slippage_diff = execution_price - market_price;
        self.analytics.total_slippage += slippage_diff * order.size;

        match order.direction {
            OrderDirection::Buy => {
                let total_cost = order.size * execution_price;
                let fees = self.calculate_fees(total_cost);
                let total_spent = total_cost + fees;

                if self.cash >= total_spent {
                    self.cash -= total_spent;
                    self.analytics.total_fees += fees;

                    let position = self
                        .portfolio
                        .entry(order.asset.clone())
                        .or_insert_with(|| Position::new(0.0, execution_price));

                    position.update(order.size, execution_price);
                    Ok(())
                } else {
                    Err("Not enough cash".to_string())
                }
            }
            OrderDirection::Sell => {
                let total_raw_value = order.size * execution_price;
                let fees = self.calculate_fees(total_raw_value);
                let total_value = total_raw_value - fees;

                let Some(position) = self.portfolio.get_mut(&order.asset) else {
                    return Err("Position not found in portfolio".to_string());
                };

                if position.quantity < order.size {
                    return Err("Not enough quantity to sell".to_string());
                }

                position.remove(order.size)?;
                self.cash += total_value;
                self.analytics.total_fees += fees;

                if position.quantity == 0.0 {
                    self.portfolio.remove(&order.asset);
                }
                Ok(())
            }
        }
    }

    // Return the total value of all the positions at the current market price
    pub fn portfolio_value(&self, data: &OHLCVData) -> f64 {
        let mut total_value = 0.0;

        for position in self.portfolio.values() {
            let current_price = data.close;
            total_value += position.quantity * current_price;
            //println!(
            //    "Asset: {}, Quantity: {}, Price: {}",
            //    asset, position.quantity, current_price
            //);
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

    fn create_dummy_date(date: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S").expect("Invalid date")
    }

    #[test]
    fn is_order_placed() {
        let mut broker = Broker::new();
        let order = Order {
            asset: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            size: 1.0,
            order_type: OrderType::Market,
            valid_until: None,
        };
        broker.place_order(order);

        assert_eq!(broker.analytics.total_placed_orders, 1);
        assert_eq!(broker.analytics.total_exec_orders, 0);
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
            valid_until: None,
        };
        broker.set_cash(1000.0);
        broker.set_fees(FeeType::Flat(1.0));
        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(100.0, 101.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&create_dummy_date("1999-11-01 00:00:00"), &dummy_price);

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
            valid_until: None,
        };
        broker.set_fees(FeeType::Flat(1.0));
        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(100.0, 101.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&create_dummy_date("1999-11-01 00:00:00"), &dummy_price);

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
            valid_until: None,
        };
        broker.set_cash(1000.0);
        broker.set_fees(FeeType::Flat(1.0));
        broker
            .portfolio
            .insert("AAPL".to_string(), Position::new(1.0, 100.0));

        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(110.0, 111.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&create_dummy_date("1999-11-01 00:00:00"), &dummy_price);

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
            valid_until: None,
        };
        broker.set_cash(1000.0);
        broker.set_fees(FeeType::Flat(1.0));
        broker
            .portfolio
            .insert("AAPL".to_string(), Position::new(1.0, 100.0));

        broker.place_order(order);

        // Simulate next tick
        let dummy_price = create_dummy_price(100.0, 101.0, 98.0, 99.0);
        broker.handle_unfulfilled_orders(&create_dummy_date("1999-11-01 00:00:00"), &dummy_price);

        // Check the cash after execution (1000 + 100 - 1 (cash + position - fee))
        assert_eq!(broker.cash, 1099.0);

        assert_eq!(broker.portfolio.len(), 0);
    }
}
