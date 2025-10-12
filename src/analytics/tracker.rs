use super::trade::{Trade, TradeDirection};
use chrono::NaiveDateTime;
use std::collections::HashMap;

pub struct TradeTracker {
    open_trades: HashMap<String, Vec<Trade>>,
    closed_trades: Vec<Trade>,
    next_trade_id: u64,
    equity_curve: Vec<(NaiveDateTime, f64)>,
    pub initial_capital: f64,
    pub total_fees: f64,
    pub total_slippage: f64,
}

impl TradeTracker {
    pub fn new() -> Self {
        TradeTracker {
            open_trades: HashMap::new(),
            closed_trades: Vec::new(),
            next_trade_id: 1,
            equity_curve: Vec::new(),
            initial_capital: 0.0,
            total_fees: 0.0,
            total_slippage: 0.0,
        }
    }

    pub fn set_initial_capital(&mut self, capital: f64) {
        self.initial_capital = capital;
    }

    pub fn record_buy(
        &mut self,
        asset: &str,
        time: NaiveDateTime,
        price: f64,
        quantity: f64,
        fees: f64,
        slippage: f64,
    ) {
        self.total_fees += fees;
        self.total_slippage += slippage * quantity;

        let trade = Trade::new(
            self.next_trade_id,
            asset.to_string(),
            time,
            price,
            quantity,
            fees,
            slippage,
            TradeDirection::Long,
        );

        self.next_trade_id += 1;

        self.open_trades
            .entry(asset.to_string())
            .or_default()
            .push(trade);
    }

    pub fn record_sell(
        &mut self,
        asset: &str,
        time: NaiveDateTime,
        price: f64,
        quantity: f64,
        fees: f64,
        slippage: f64,
    ) {
        self.total_fees += fees;
        self.total_slippage += slippage * quantity;

        let open_positions = match self.open_trades.get_mut(asset) {
            Some(positions) => positions,
            None => return,
        };

        let total_fees = fees;
        let mut remaining_quantity = quantity;
        let mut trades_to_close = Vec::new();

        for (idx, trade) in open_positions.iter_mut().enumerate() {
            if remaining_quantity <= 0.0 {
                break;
            }

            let quantity_to_close = remaining_quantity.min(trade.quantity);
            let fee_proportion = quantity_to_close / quantity;

            if quantity_to_close >= trade.quantity {
                trade.close(
                    time,
                    price,
                    total_fees * fee_proportion,
                    slippage * fee_proportion,
                );
                trades_to_close.push(idx);
            } else {
                let mut closed_trade = trade.clone();
                closed_trade.quantity = quantity_to_close;
                let closed_entry_fees = trade.entry_fees * (quantity_to_close / trade.quantity);
                closed_trade.entry_fees = closed_entry_fees;
                closed_trade.close(
                    time,
                    price,
                    total_fees * fee_proportion,
                    slippage * fee_proportion,
                );
                self.closed_trades.push(closed_trade);

                trade.quantity -= quantity_to_close;
                trade.entry_fees -= closed_entry_fees;
            }

            remaining_quantity -= quantity_to_close;
        }

        for &idx in trades_to_close.iter().rev() {
            let trade = open_positions.remove(idx);
            self.closed_trades.push(trade);
        }

        if open_positions.is_empty() {
            self.open_trades.remove(asset);
        }
    }

    pub fn record_equity_snapshot(&mut self, time: NaiveDateTime, total_value: f64) {
        self.equity_curve.push((time, total_value));
    }

    pub fn get_closed_trades(&self) -> &[Trade] {
        &self.closed_trades
    }

    pub fn get_equity_curve(&self) -> &[(NaiveDateTime, f64)] {
        &self.equity_curve
    }
}
