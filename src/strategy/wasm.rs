use crate::broker::order::{Order, OrderDirection, OrderType};
use crate::broker::Broker;
use crate::data::OHLCVData;
use crate::strategy::Strategy;
use chrono::NaiveDateTime;
use std::ptr;
use wasmtime::*;

pub struct WasmStrategy {
    _engine: Engine,
    store: Store<HostState>,
    _instance: Instance,
    init_fn: TypedFunc<(), ()>,
    tick_fn: TypedFunc<(i64, f64, f64, f64, f64, f64), ()>,
}

struct HostState {
    broker_ptr: *mut Broker,
    memory: Option<Memory>,
}

unsafe impl Send for HostState {}

fn read_string_from_memory(caller: &Caller<'_, HostState>, ptr: i32, len: i32) -> String {
    let memory = caller.data().memory.unwrap();
    let data = memory.data(caller);
    let bytes = &data[ptr as usize..(ptr + len) as usize];
    String::from_utf8_lossy(bytes).to_string()
}

impl WasmStrategy {
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Config::new();
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, wasm_bytes)?;

        let host_state = HostState {
            broker_ptr: ptr::null_mut(),
            memory: None,
        };

        let mut store = Store::new(&engine, host_state);

        let mut linker = Linker::new(&engine);

        let memory_ty = MemoryType::new(16, Some(256));
        let memory = Memory::new(&mut store, memory_ty)?;
        linker.define(&store, "env", "memory", memory)?;

        store.data_mut().memory = Some(memory);

        linker.func_wrap(
            "env",
            "place_market_order",
            |mut caller: Caller<'_, HostState>,
             asset_ptr: i32,
             asset_len: i32,
             direction: i32,
             size: f64| {
                let asset = read_string_from_memory(&caller, asset_ptr, asset_len);

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

                unsafe {
                    let broker = &mut *caller.data_mut().broker_ptr;
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
                let asset = read_string_from_memory(&caller, asset_ptr, asset_len);

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

                unsafe {
                    let broker = &mut *caller.data_mut().broker_ptr;
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
                let asset = read_string_from_memory(&caller, asset_ptr, asset_len);

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

                unsafe {
                    let broker = &mut *caller.data_mut().broker_ptr;
                    broker.place_order(order);
                }
            },
        )?;

        linker.func_wrap("env", "get_cash", |caller: Caller<'_, HostState>| -> f64 {
            unsafe {
                let broker = &*caller.data().broker_ptr;
                broker.cash
            }
        })?;

        linker.func_wrap(
            "env",
            "get_position",
            |caller: Caller<'_, HostState>, asset_ptr: i32, asset_len: i32| -> f64 {
                let asset = read_string_from_memory(&caller, asset_ptr, asset_len);

                unsafe {
                    let broker = &*caller.data().broker_ptr;
                    broker
                        .portfolio
                        .get(&asset)
                        .map(|p| p.quantity)
                        .unwrap_or(0.0)
                }
            },
        )?;

        linker.func_wrap(
            "env",
            "log",
            |caller: Caller<'_, HostState>, ptr: i32, len: i32| {
                let memory = caller.data().memory.unwrap();
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
            store,
            _instance: instance,
            init_fn,
            tick_fn,
        })
    }
}

impl Strategy for WasmStrategy {
    fn init(&mut self) {
        self.init_fn.call(&mut self.store, ()).ok();
    }

    fn tick(
        &mut self,
        current_time: &NaiveDateTime,
        data: Option<&OHLCVData>,
        broker: &mut Broker,
    ) {
        self.store.data_mut().broker_ptr = broker as *mut Broker;

        if let Some(current) = data {
            let timestamp = current_time.and_utc().timestamp();
            self.tick_fn
                .call(
                    &mut self.store,
                    (
                        timestamp,
                        current.open,
                        current.high,
                        current.low,
                        current.close,
                        current.volume as f64,
                    ),
                )
                .ok();
        }

        self.store.data_mut().broker_ptr = ptr::null_mut();
    }
}
