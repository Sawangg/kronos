use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum TradeDirection {
    Long,
}

// Complete order (buy + sell)
#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    pub id: u64,
    pub asset: String,
    pub entry_time: NaiveDateTime,
    pub entry_price: f64,
    pub quantity: f64,
    pub entry_fees: f64,
    pub entry_slippage: f64,
    pub exit_time: Option<NaiveDateTime>,
    pub exit_price: Option<f64>,
    pub exit_fees: f64,
    pub exit_slippage: f64,
    pub profit_loss: Option<f64>,
    pub return_pct: Option<f64>,
    pub direction: TradeDirection,
}

impl Trade {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        asset: String,
        entry_time: NaiveDateTime,
        entry_price: f64,
        quantity: f64,
        entry_fees: f64,
        entry_slippage: f64,
        direction: TradeDirection,
    ) -> Self {
        Trade {
            id,
            asset,
            entry_time,
            entry_price,
            quantity,
            entry_fees,
            entry_slippage,
            exit_time: None,
            exit_price: None,
            exit_fees: 0.0,
            exit_slippage: 0.0,
            profit_loss: None,
            return_pct: None,
            direction,
        }
    }

    pub fn close(
        &mut self,
        exit_time: NaiveDateTime,
        exit_price: f64,
        exit_fees: f64,
        exit_slippage: f64,
    ) {
        self.exit_time = Some(exit_time);
        self.exit_price = Some(exit_price);
        self.exit_fees = exit_fees;
        self.exit_slippage = exit_slippage;

        let entry_cost = self.entry_price * self.quantity + self.entry_fees;
        let exit_value = exit_price * self.quantity - exit_fees;

        match self.direction {
            TradeDirection::Long => {
                self.profit_loss = Some(exit_value - entry_cost);
            }
        }

        if let Some(pl) = self.profit_loss {
            self.return_pct = Some((pl / entry_cost) * 100.0);
        }
    }
}
