# Kronos

Fast backtest framework to test your quantitative strategies. Here is a list of some of the features:

- Fast and easy to use (3 seconds to backtest 1 year of data with a tick of 1 second)
- Data source agnostic, provide OHLCV data how you wish
- Can be used with a variety of assets (Stocks, Crypto, ...)
- Able to simulate down to a precision of 1 nanosecond for HFT strategies

## TODO

- Support for derivatives (Futures, Options, Warrants, ...)
- Configurable slippage for a more realistic result
- Supports multiple instruments in one simulation for complex strategies
- Visualize your strategy how you wish using websockets
- Support multiple programming languages to create your strategy
- Support L2 data for a better overview of the order book
- Support other data types like FIX

> [!NOTE]
> You should be careful about stock split in your data if it isn't ajusted, it might falsify the result of the simulation

## Implement a strategy

This is how you implement a test strategy using `Kronos`.

```rs
pub struct TestStrategy;

impl Strategy for TestStrategy {
    // Called once at the beginning of the execution
    fn init(&mut self) {}

    // Called once every tick (the time between ticks can be of any duration, by default it is 1m)
    fn next(&mut self, current_time: &NaiveDateTime, data: &[OHLCVData], broker: &mut Broker) {
        // Here is how you place a market order
        let order = Order {
            instrument: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            order_type: OrderType::Market,
            size: 1.0,
        };
        broker.place_order(order);
    }
}
```
