use engine_core::MatchingEngine;
use engine_core::Orderbook;
use engine_core::Side;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct WasmEngine {
    inner: MatchingEngine,
}

#[wasm_bindgen]
pub enum WasmSide {
    Buy,
    Sell,
}

impl From<WasmSide> for Side {
    fn from(value: WasmSide) -> Side {
        match value {
            WasmSide::Buy => Side::Buy,
            WasmSide::Sell => Side::Sell,
        }
    }
}
#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        let orderbook = Orderbook::new();

        WasmEngine {
            inner: MatchingEngine::new(orderbook),
        }
    }

    pub fn place_limit_order(&mut self, price: u64, qty: u64, side: WasmSide) {
        let side: Side = side.into();

        self.inner.place_limit_order(price, qty, side);
    }

    pub fn place_market_order(&mut self, qty: u64, side: WasmSide) -> Vec<u64> {
        let (id, filled) = self.inner.place_market_order(qty, side.into());
        vec![id, filled]
    }

    pub fn trades(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.trades)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    pub fn orderbook_full_state(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner).map_err(|e| JsValue::from_str(&e.to_string()))
    }
    pub fn orderbook_depth_state(&self) -> Result<JsValue, JsValue> {
        let snapshot = self.inner.depth_snapshot();
        serde_wasm_bindgen::to_value(&snapshot).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn drain_trades(&mut self) -> Result<JsValue, JsValue> {
        let drained = self.inner.drain_trades();
        serde_wasm_bindgen::to_value(&drained).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn price_scale() -> u64 {
        engine_core::MatchingEngine::PRICE_SCALE
    }
}

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}
