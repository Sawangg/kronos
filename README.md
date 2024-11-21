# Kronos

Fastest backtest and paper trade framework to test your quantitative strategy. Here is a list of some features:

- Fast and easy to use
- Data source agnostic
- Supports multiple tickers for complex strategies

## TODO

- Visualize your strategy how you wish using your own frontend and websocket
- Support multiple programming languages to create your strategy
- Paper trading using live data sources

## Info

Orders are executed on the open of the next tick, meaning you can have a bit of slipage between the time you placed the
order and its execution.

## Implement a strategy

This is how you implement a test strategy using `Kronos`.

```rs
pub struct TestStrategy;

impl Strategy for TestStrategy {
    // Called once at the beginning of the execution
    fn init(&mut self) {}

    // Called once every new tick (a tick can be a second or a day depending on the provided data)
    fn next(&mut self, data: &[OHLCVData], broker: &mut Broker) {
        // Here is how you place a market order
        let order = Order {
            asset: "AAPL".to_string(),
            direction: OrderDirection::Buy,
            order_type: OrderType::Market,
            size: 1.0,
        };
        broker.place_order(order);
    }

    // Called once every new tick
    fn log(&self) {}
}
```

## Warning

You should be careful about stock split in your data if it isn't ajusted, it might falsify the result of the simulation
