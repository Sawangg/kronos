use super::trade::Trade;
use crate::broker::fee::FeeType;
use chrono::{Duration, NaiveDateTime};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GlobalMetrics {
    pub cash: f64,
    pub portfolio_value: f64,
    pub total_equity: f64,
    pub gross_profit: f64,
    pub total_fees: f64,
    pub total_slippage: f64,
    pub net_profit: f64,
    pub net_profit_percentage: f64,
    pub num_orders_placed: i32,
    pub num_orders_executed: i32,
    pub roi: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub max_drawdown_duration_days: i64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub largest_win: f64,
    pub largest_loss: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub avg_trade_duration_hours: f64,
    pub buy_hold_roi: f64,
    pub buy_hold_final_value: f64,
    pub buy_hold_net_profit: f64,
}

impl GlobalMetrics {
    pub fn calculate(
        trades: &[Trade],
        equity_curve: &[(NaiveDateTime, f64)],
        initial_capital: f64,
        risk_free_rate: f64,
        cash: f64,
        portfolio_value: f64,
        num_orders_placed: i32,
        num_orders_executed: i32,
        total_fees: f64,
        total_slippage: f64,
        first_price: Option<f64>,
        last_price: Option<f64>,
        fee_type: &Option<FeeType>,
    ) -> Self {
        if trades.is_empty() {
            return Self::default();
        }

        let total_trades = trades.len();
        let winning_trades: Vec<_> = trades
            .iter()
            .filter(|t| t.profit_loss.unwrap_or(0.0) > 0.0)
            .collect();
        let losing_trades: Vec<_> = trades
            .iter()
            .filter(|t| t.profit_loss.unwrap_or(0.0) < 0.0)
            .collect();

        let total_profit: f64 = winning_trades
            .iter()
            .map(|t| t.profit_loss.unwrap_or(0.0))
            .sum();
        let total_loss: f64 = losing_trades
            .iter()
            .map(|t| t.profit_loss.unwrap_or(0.0).abs())
            .sum();

        let win_rate = if total_trades > 0 {
            (winning_trades.len() as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        let profit_factor = if total_loss > 0.0 {
            total_profit / total_loss
        } else if total_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        let avg_win = if !winning_trades.is_empty() {
            total_profit / winning_trades.len() as f64
        } else {
            0.0
        };

        let avg_loss = if !losing_trades.is_empty() {
            -total_loss / losing_trades.len() as f64
        } else {
            0.0
        };

        let largest_win = winning_trades
            .iter()
            .map(|t| t.profit_loss.unwrap_or(0.0))
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let largest_loss = losing_trades
            .iter()
            .map(|t| t.profit_loss.unwrap_or(0.0))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let final_value = equity_curve
            .last()
            .map(|(_, v)| *v)
            .unwrap_or(initial_capital);
        let roi = ((final_value - initial_capital) / initial_capital) * 100.0;

        let sharpe_ratio = Self::calculate_sharpe_ratio(equity_curve, risk_free_rate);

        let (max_drawdown, max_drawdown_duration_days) = Self::calculate_max_drawdown(equity_curve);

        let avg_trade_duration_hours = if !trades.is_empty() {
            let total_duration: i64 = trades
                .iter()
                .filter_map(|t| {
                    if let (Some(exit_time), entry_time) = (t.exit_time, t.entry_time) {
                        Some((exit_time - entry_time).num_hours())
                    } else {
                        None
                    }
                })
                .sum();
            total_duration as f64 / trades.len() as f64
        } else {
            0.0
        };

        let total_equity = cash + portfolio_value;
        let gross_profit = total_equity - initial_capital;
        let net_profit = gross_profit - total_fees - total_slippage;
        let net_profit_percentage = if initial_capital > 0.0 {
            (net_profit / initial_capital) * 100.0
        } else {
            0.0
        };

        let (buy_hold_roi, buy_hold_final_value, buy_hold_net_profit) =
            if let (Some(first), Some(last)) = (first_price, last_price) {
                Self::calculate_buy_and_hold(initial_capital, first, last, fee_type)
            } else {
                (0.0, 0.0, 0.0)
            };

        GlobalMetrics {
            cash: f64::trunc(cash * 100.0) / 100.0,
            portfolio_value: f64::trunc(portfolio_value * 100.0) / 100.0,
            total_equity: f64::trunc(total_equity * 100.0) / 100.0,
            gross_profit: f64::trunc(gross_profit * 100.0) / 100.0,
            total_fees: f64::trunc(total_fees * 100.0) / 100.0,
            total_slippage: f64::trunc(total_slippage * 100.0) / 100.0,
            net_profit: f64::trunc(net_profit * 100.0) / 100.0,
            net_profit_percentage: f64::trunc(net_profit_percentage * 100.0) / 100.0,
            num_orders_placed,
            num_orders_executed,
            roi,
            sharpe_ratio,
            max_drawdown,
            max_drawdown_duration_days,
            win_rate,
            profit_factor,
            avg_win,
            avg_loss,
            largest_win,
            largest_loss,
            total_trades,
            winning_trades: winning_trades.len(),
            losing_trades: losing_trades.len(),
            avg_trade_duration_hours,
            buy_hold_roi: f64::trunc(buy_hold_roi * 100.0) / 100.0,
            buy_hold_final_value: f64::trunc(buy_hold_final_value * 100.0) / 100.0,
            buy_hold_net_profit: f64::trunc(buy_hold_net_profit * 100.0) / 100.0,
        }
    }

    fn calculate_sharpe_ratio(equity_curve: &[(NaiveDateTime, f64)], risk_free_rate: f64) -> f64 {
        if equity_curve.len() < 2 {
            return 0.0;
        }

        let returns: Vec<f64> = equity_curve
            .windows(2)
            .map(|w| {
                let (_, prev_value) = w[0];
                let (_, curr_value) = w[1];
                (curr_value - prev_value) / prev_value
            })
            .collect();

        if returns.is_empty() {
            return 0.0;
        }

        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>()
            / returns.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            return 0.0;
        }

        let daily_risk_free = risk_free_rate / 252.0;
        let sharpe = (mean_return - daily_risk_free) / std_dev;

        sharpe * (252.0_f64).sqrt()
    }

    fn calculate_max_drawdown(equity_curve: &[(NaiveDateTime, f64)]) -> (f64, i64) {
        if equity_curve.is_empty() {
            return (0.0, 0);
        }

        let mut max_value = equity_curve[0].1;
        let mut max_drawdown = 0.0;
        let mut max_drawdown_duration = Duration::zero();
        let mut drawdown_start: Option<NaiveDateTime> = None;

        for &(time, value) in equity_curve.iter() {
            if value > max_value {
                max_value = value;
                drawdown_start = None;
            } else {
                let drawdown = ((value - max_value) / max_value) * 100.0;
                if drawdown < max_drawdown {
                    max_drawdown = drawdown;
                }

                if drawdown_start.is_none() {
                    drawdown_start = Some(time);
                }

                if let Some(start) = drawdown_start {
                    let duration = time - start;
                    if duration > max_drawdown_duration {
                        max_drawdown_duration = duration;
                    }
                }
            }
        }

        (max_drawdown, max_drawdown_duration.num_days())
    }

    fn calculate_buy_and_hold(
        initial_capital: f64,
        first_price: f64,
        last_price: f64,
        fee_type: &Option<FeeType>,
    ) -> (f64, f64, f64) {
        if first_price <= 0.0 || last_price <= 0.0 || initial_capital <= 0.0 {
            return (0.0, 0.0, 0.0);
        }

        let buy_fee = match fee_type {
            Some(FeeType::Flat(fee)) => *fee,
            Some(FeeType::Percentage(percentage)) => initial_capital * percentage,
            None => 0.0,
        };

        let capital_after_buy_fee = initial_capital - buy_fee;
        if capital_after_buy_fee <= 0.0 {
            return (0.0, 0.0, 0.0);
        }

        let shares = capital_after_buy_fee / first_price;
        let value_before_sell = shares * last_price;

        let sell_fee = match fee_type {
            Some(FeeType::Flat(fee)) => *fee,
            Some(FeeType::Percentage(percentage)) => value_before_sell * percentage,
            None => 0.0,
        };

        let final_value = value_before_sell - sell_fee;
        let net_profit = final_value - initial_capital;
        let roi = (net_profit / initial_capital) * 100.0;

        (roi, final_value, net_profit)
    }
}

impl Default for GlobalMetrics {
    fn default() -> Self {
        GlobalMetrics {
            cash: 0.0,
            portfolio_value: 0.0,
            total_equity: 0.0,
            gross_profit: 0.0,
            total_fees: 0.0,
            total_slippage: 0.0,
            net_profit: 0.0,
            net_profit_percentage: 0.0,
            num_orders_placed: 0,
            num_orders_executed: 0,
            roi: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            max_drawdown_duration_days: 0,
            win_rate: 0.0,
            profit_factor: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            largest_win: 0.0,
            largest_loss: 0.0,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            avg_trade_duration_hours: 0.0,
            buy_hold_roi: 0.0,
            buy_hold_final_value: 0.0,
            buy_hold_net_profit: 0.0,
        }
    }
}
