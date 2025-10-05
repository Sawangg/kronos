use crate::broker::broker::Broker;
use crate::broker::order::{Order, OrderDirection, OrderType};
use crate::data::OHLCVData;
use crate::strategy::Strategy;
use chrono::NaiveDateTime;
use std::sync::{Arc, Mutex};
use wasmtime::*;

pub struct WasmStrategy {
    _engine: Engine,
    _module: Module,
    store: Store<HostState>,
    _instance: Instance,
    init_fn: TypedFunc<(), ()>,
    tick_fn: TypedFunc<(i64, f64, f64, f64, f64, f64), ()>,
}

struct HostState {
    broker: Arc<Mutex<Option<Broker>>>,
}

impl WasmStrategy {
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;

        let host_state = HostState {
            broker: Arc::new(Mutex::new(None)),
        };

        let mut store = Store::new(&engine, host_state);

        let mut linker = Linker::new(&engine);

        let memory_ty = MemoryType::new(16, Some(256));
        let memory = Memory::new(&mut store, memory_ty)?;
        linker.define(&store, "env", "memory", memory)?;

        linker.func_wrap(
            "env",
            "place_market_order",
            |mut caller: Caller<'_, HostState>,
             asset_ptr: i32,
             asset_len: i32,
             direction: i32,
             size: f64| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let data = memory.data(&caller);

                let asset_bytes = &data[asset_ptr as usize..(asset_ptr + asset_len) as usize];
                let asset = String::from_utf8_lossy(asset_bytes).to_string();

                let order_direction = match direction {
                    0 => OrderDirection::Buy,
                    1 => OrderDirection::Sell,
                    _ => return,
                };

                let order = Order {
                    asset,
                    direction: order_direction,
                    order_type: OrderType::Market,
                    size,
                    valid_until: None,
                };

                if let Some(ref mut broker) = *caller.data_mut().broker.lock().unwrap() {
                    broker.place_order(order);
                }
            },
        )?;

        linker.func_wrap(
            "env",
            "place_limit_order",
            |mut caller: Caller<'_, HostState>,
             asset_ptr: i32,
             asset_len: i32,
             direction: i32,
             size: f64,
             price: f64| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let data = memory.data(&caller);

                let asset_bytes = &data[asset_ptr as usize..(asset_ptr + asset_len) as usize];
                let asset = String::from_utf8_lossy(asset_bytes).to_string();

                let order_direction = match direction {
                    0 => OrderDirection::Buy,
                    1 => OrderDirection::Sell,
                    _ => return,
                };

                let order = Order {
                    asset,
                    direction: order_direction,
                    order_type: OrderType::Limit(price),
                    size,
                    valid_until: None,
                };

                if let Some(ref mut broker) = *caller.data_mut().broker.lock().unwrap() {
                    broker.place_order(order);
                }
            },
        )?;

        linker.func_wrap(
            "env",
            "place_stop_order",
            |mut caller: Caller<'_, HostState>,
             asset_ptr: i32,
             asset_len: i32,
             direction: i32,
             size: f64,
             stop_price: f64| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let data = memory.data(&caller);

                let asset_bytes = &data[asset_ptr as usize..(asset_ptr + asset_len) as usize];
                let asset = String::from_utf8_lossy(asset_bytes).to_string();

                let order_direction = match direction {
                    0 => OrderDirection::Buy,
                    1 => OrderDirection::Sell,
                    _ => return,
                };

                let order = Order {
                    asset,
                    direction: order_direction,
                    order_type: OrderType::Stop(stop_price),
                    size,
                    valid_until: None,
                };

                if let Some(ref mut broker) = *caller.data_mut().broker.lock().unwrap() {
                    broker.place_order(order);
                }
            },
        )?;

        linker.func_wrap("env", "get_cash", |caller: Caller<'_, HostState>| -> f64 {
            if let Some(ref broker) = *caller.data().broker.lock().unwrap() {
                broker.cash
            } else {
                0.0
            }
        })?;

        linker.func_wrap(
            "env",
            "get_position",
            |mut caller: Caller<'_, HostState>, asset_ptr: i32, asset_len: i32| -> f64 {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let data = memory.data(&caller);

                let asset_bytes = &data[asset_ptr as usize..(asset_ptr + asset_len) as usize];
                let asset = String::from_utf8_lossy(asset_bytes).to_string();

                if let Some(ref broker) = *caller.data().broker.lock().unwrap() {
                    broker
                        .portfolio
                        .get(&asset)
                        .map(|p| p.quantity)
                        .unwrap_or(0.0)
                } else {
                    0.0
                }
            },
        )?;

        linker.func_wrap(
            "env",
            "log",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let data = memory.data(&caller);

                let bytes = &data[ptr as usize..(ptr + len) as usize];
                let message = String::from_utf8_lossy(bytes);
                println!("[WASM]: {}", message);
            },
        )?;

        linker.func_wrap(
            "env",
            "abort",
            |_: Caller<'_, HostState>, _: i32, _: i32, _: i32, _: i32| {
                eprintln!("[WASM]: abort called");
            },
        )?;

        let instance = linker.instantiate(&mut store, &module)?;

        let init_fn = instance.get_typed_func::<(), ()>(&mut store, "init")?;
        let tick_fn =
            instance.get_typed_func::<(i64, f64, f64, f64, f64, f64), ()>(&mut store, "tick")?;

        Ok(WasmStrategy {
            _engine: engine,
            _module: module,
            store,
            _instance: instance,
            init_fn,
            tick_fn,
        })
    }

    #[allow(dead_code)]
    pub fn set_broker(&mut self, broker: Broker) {
        *self.store.data_mut().broker.lock().unwrap() = Some(broker);
    }

    #[allow(dead_code)]
    pub fn take_broker(&mut self) -> Option<Broker> {
        self.store.data_mut().broker.lock().unwrap().take()
    }
}

impl Strategy for WasmStrategy {
    fn init(&mut self) {
        self.init_fn.call(&mut self.store, ()).ok();
    }

    fn tick(&mut self, current_time: &NaiveDateTime, data: &[OHLCVData], broker: &mut Broker) {
        *self.store.data_mut().broker.lock().unwrap() =
            Some(std::mem::replace(broker, Broker::new()));

        if let Some(latest) = data.last() {
            let timestamp = current_time.and_utc().timestamp();
            self.tick_fn
                .call(
                    &mut self.store,
                    (
                        timestamp,
                        latest.open,
                        latest.high,
                        latest.low,
                        latest.close,
                        latest.volume as f64,
                    ),
                )
                .ok();
        }

        if let Some(updated_broker) = self.store.data_mut().broker.lock().unwrap().take() {
            *broker = updated_broker;
        }
    }
}
