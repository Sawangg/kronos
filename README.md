# Kronos

Fast and versatile backtest framework to test your quantitative strategies. Here is a list of some of the features:

- Implement your strategy with any language that compiles to WebAssembly (Rust, Python, Typescript, C/C++, ...)
- Data source agnostic, can be used with a variety of assets (stocks, crypto, ...) by providing OHLCV data how you wish
- Fast (less than 10 seconds to backtest 1 year of data with a tick of 1 second which is around 31 million data points)
- Easy to use, send a simple POST request to start your backtest
- Place market orders, limit orders and stop orders
- Able to simulate down to a precision of 1 nanosecond for HFT strategies
- Detailed metrics about your strategy performance with per trade analysis
- Configurable slippage for a more realistic result

> [!IMPORTANT]
> You should be careful about stock split in your data. If it isn't ajusted, it might falsify the result of the simulation.

## Usage

Here is an example of an execution of a strategy with a simple POST request

```sh
curl -i http://localhost:3000/run -H "Content-Type: application/json" -d '{
    "parameters": {
      "start_date": "2024-02-17 00:00:00",
      "end_date": "2025-02-17 00:00:00",
      "tick": "1m"
    },
    "data": {
      "source": "..."
    },
    "broker": {
      "cash": 10000.0,
      "fees": { "Flat": 1.0 },
      "slippage": {
        "min": 0.01,
        "max": 0.05
      }
    },
    "strategy": {
      "wasm": "..."
    }
  }'
```

## Ideas and TODO

- Benchmark strategy against a buy and hold strategy & indexes on the same period
- Calculate taxes impact on your strategy performance
- Visualize your strategy using a dedicated frontend
- Support for derivatives (Futures, Options, Warrants, ...)
- Support for Stop-Limit orders
- Support multiple instruments in one simulation for complex strategies
- Support L2 data for a better overview of the order book with spread visualization etc
- Support other data types like FIX
- Support for better splippage predictions using liquidity and volatility
- Support paper trading with live market data
- Support for stress testing during significant market events
- Replay system with per trade visual and total history
- Better time management with timezones etc
