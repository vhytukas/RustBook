use engine_core::simulation::Simulator;
use engine_core::{MatchingEngine, Orderbook, RiskGate, Side};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmEngine {
    pub(crate) inner: MatchingEngine,
    pub(crate) simulator: Option<Simulator>,
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

fn to_js_err<E: std::fmt::Debug>(e: E) -> JsValue {
    JsValue::from_str(&format!("{:?}", e))
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        let mut inner = MatchingEngine::new(Orderbook::new());
        inner.set_risk_gate(RiskGate::live_defaults());
        WasmEngine {
            inner,
            simulator: None,
        }
    }

    pub fn default_risk_gate() -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&RiskGate::live_defaults()).map_err(to_js_err)
    }

    pub fn place_limit_order(
        &mut self,
        price: u64,
        qty: u64,
        side: WasmSide,
    ) -> Result<u64, JsValue> {
        self.inner
            .place_limit_order(price, qty, side.into())
            .map_err(to_js_err)
    }

    pub fn place_market_order(&mut self, qty: u64, side: WasmSide) -> Result<Vec<u64>, JsValue> {
        let (id, filled) = self
            .inner
            .place_market_order(qty, side.into())
            .map_err(to_js_err)?;
        Ok(vec![id, filled])
    }

    pub fn cancel_order(&mut self, order_id: u64) -> Result<u64, JsValue> {
        self.inner.cancel_order(order_id).map_err(to_js_err)
    }

    pub fn amend_order_qty(&mut self, order_id: u64, new_qty: u64) -> Result<(), JsValue> {
        self.inner
            .amend_order_qty(order_id, new_qty)
            .map_err(to_js_err)
    }

    pub fn set_risk_gate(&mut self, config_js: JsValue) -> Result<(), JsValue> {
        let config: RiskGate = serde_wasm_bindgen::from_value(config_js)?;
        self.inner.set_risk_gate(config);
        Ok(())
    }

    pub fn clear_risk_gate(&mut self) {
        self.inner.set_risk_gate(RiskGate::default());
    }

    pub fn trades(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.trades).map_err(to_js_err)
    }

    pub fn drain_trades(&mut self) -> Result<JsValue, JsValue> {
        let drained = self.inner.drain_trades();
        serde_wasm_bindgen::to_value(&drained).map_err(to_js_err)
    }

    pub fn orderbook_full_state(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner).map_err(to_js_err)
    }

    pub fn orderbook_depth_state(&self) -> Result<JsValue, JsValue> {
        let snapshot = self.inner.depth_snapshot();
        serde_wasm_bindgen::to_value(&snapshot).map_err(to_js_err)
    }

    #[wasm_bindgen]
    pub fn price_scale() -> u64 {
        MatchingEngine::PRICE_SCALE
    }
}
