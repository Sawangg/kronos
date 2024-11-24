# Kronos

Fastest backtest and paper trade framework to test your quantitative strategy. Here is a list of some features:

- Fast and easy to use (1 second to backtest 1 year of data with a tick of 1 second)
- Data source agnostic
- Can simulate down to a precision of 1 nanosecond

## TODO

- Supports multiple tickers for complex strategies
- Visualize your strategy how you wish using your own frontend and websocket
- Support multiple programming languages to create your strategy
- Paper trading using live data sources

## How to run

```rs

```

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
            asset: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            order_type: OrderType::Market,
            size: 1.0,
        };
        broker.place_order(order);
    }
}
```

## Warning

You should be careful about stock split in your data if it isn't ajusted, it might falsify the result of the simulation
