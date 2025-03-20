# Kronos

Fast and versatile backtest framework to test your quantitative strategies. Here is a list of some of the features:

- Fast (less than 3 seconds to backtest 1 year of data with a tick of 1 second)
- Easy to use, send a simple POST request to start your backtest
- Can be used with a variety of assets (Stocks, Crypto, ...)
- Place market orders, limit orders and stop orders
- Able to simulate down to a precision of 1 nanosecond for HFT strategies
- Configurable slippage for a more realistic result

> [!IMPORTANT]
> You should be careful about stock split in your data. If it isn't ajusted, it might falsify the result of the simulation.

## Usage

Here is an example of an execution of a strategy with a simple POST request

```sh
curl -i http://localhost:3000/run -H "Content-Type: application/json" -d '{
    "parameters": {
      "start_date": "2024-02-17",
      "end_date": "2025-02-17",
      "tick": "1s"
    },
    "data": "AAPL",
    "broker": {
      "cash": 10000.0,
      "fees": { "Flat": 1.0 }
    }
  }'
```

## Ideas and TODO

- Implement your strategy with any language that compiles to WASM
- Data source agnostic, provide OHLCV data how you wish
- Better analytics result with per trade feedback
- Visualize your strategy using a dedicated frontend. Analyze key performance metrics such as:
  - Return on Investment (ROI)
  - Sharpe Ratio: Measures risk-adjusted return.
  - Max Drawdown: The largest peak-to-trough decline.
  - Win Rate: Percentage of profitable trades.
- Support for derivatives (Futures, Options, Warrants, ...)
- Support for Stop-Limit orders
- Supports multiple instruments in one simulation for complex strategies
- Support L2 data for a better overview of the order book with spread visualization etc
- Support other data types like FIX
- Support for better splippage predictions using liquidity and volatility
- Support paper trading with live market data
- Support for stress testing during significant market events
- Replay system with per trade visual and total history
- Better time management with timezones etc
