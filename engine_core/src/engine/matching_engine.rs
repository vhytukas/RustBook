use super::depth::*;
use super::order::Order;
use super::orderbook::Orderbook;
use super::side::Side;
use super::trade::Trade;
use serde::*;

#[derive(Debug, PartialEq, Eq)]
pub enum CancelError {
    OrderNotFound,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AmendError {
    OrderNotFound,
    SizeIncrease,
    InvalidQty,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RiskRejection {
    QtyExceedsLimit,
    NotionalExceedsLimit,
    PriceTooFarFromMid,
}
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RiskGate {
    pub max_order_qty: Option<u64>,
    pub max_notional: Option<u64>,
    pub max_price_deviation: Option<u64>,
}

impl RiskGate {
    /// Sensible production-leaning defaults for live use.
    /// `default()` is intentionally all-None (no checks) for test compatibility;
    /// `live_defaults()` is what the WASM engine ships with.
    ///
    /// At PRICE_SCALE=100 these mean:
    ///   - max single-order qty: 10,000 units
    ///   - max single-order notional: $1,000,000 (price × qty in scaled units)
    ///   - max price deviation from mid: $20.00 (~20% at $100 mid)
    pub fn live_defaults() -> Self {
        Self {
            max_order_qty: Some(10_000),
            max_notional: Some(100_000_000),
            max_price_deviation: Some(2_000),
        }
    }
}
#[derive(Serialize)]
pub struct MatchingEngine {
    pub book: Orderbook,
    pub trades: Vec<Trade>,
    next_id: u64,
    #[serde(skip)]
    risk: RiskGate,
}

impl MatchingEngine {
    pub const PRICE_SCALE: u64 = 100;

    pub fn new(book: Orderbook) -> MatchingEngine {
        MatchingEngine {
            book,
            trades: Vec::new(),
            next_id: 1,
            risk: RiskGate::default(),
        }
    }

    pub fn place_limit_order(
        &mut self,
        price: u64,
        qty: u64,
        side: Side,
    ) -> Result<u64, RiskRejection> {
        self.check_risk(Some(price), qty)?;

        let id: u64 = self.next_id;
        self.next_id += 1;

        let order = Order::new(id, price, qty, side);

        self.match_or_insert(order);

        Ok(id)
    }

    pub fn place_market_order(
        &mut self,
        qty: u64,
        side: Side,
    ) -> Result<(u64, u64), RiskRejection> {
        self.check_risk(None, qty)?;

        let id = self.next_id;

        self.next_id += 1;

        let extreme = match side {
            Side::Buy => u64::MAX,
            Side::Sell => 0,
        };

        let mut order = Order::new(id, extreme, qty, side);

        self.match_against_book(&mut order);

        let filled = qty - order.qty;

        Ok((id, filled))
    }

    fn match_or_insert(&mut self, mut order: Order) {
        self.match_against_book(&mut order);

        if order.qty > 0 {
            self.book.insert_order(order);
        }
    }

    fn match_against_book(&mut self, order: &mut Order) {
        while order.qty > 0 {
            let best_price = match order.side {
                Side::Buy => self.book.best_ask_price(),
                Side::Sell => self.book.best_bid_price(),
            };

            let Some(best_price) = best_price else { break };

            let crosses = match order.side {
                Side::Buy => order.price >= best_price,
                Side::Sell => order.price <= best_price,
            };
            if !crosses {
                break;
            }

            let (level_emptied, removed_ids) = {
                let level = match order.side {
                    Side::Buy => self.book.asks.get_mut(&best_price).unwrap(),
                    Side::Sell => self.book.bids.get_mut(&best_price).unwrap(),
                };

                let mut removed_ids: Vec<u64> = Vec::new();

                while order.qty > 0 {
                    let (maker_id, fill_qty, should_pop) = {
                        let Some(front) = level.orders.front_mut() else {
                            break;
                        };

                        let maker_id = front.id;
                        let fill_qty = order.qty.min(front.qty);

                        front.qty -= fill_qty;
                        let should_pop = front.qty == 0;

                        (maker_id, fill_qty, should_pop)
                    };

                    self.trades.push(Trade::new(
                        maker_id, order.id, order.side, best_price, fill_qty,
                    ));

                    order.qty -= fill_qty;

                    level.reduce_qty(fill_qty);

                    if should_pop {
                        level.orders.pop_front();
                        removed_ids.push(maker_id);
                    }
                }

                (level.orders.is_empty(), removed_ids)
            };

            for id in removed_ids {
                self.book.index.remove(&id);
            }

            if level_emptied {
                match order.side {
                    Side::Buy => {
                        self.book.asks.remove(&best_price);
                    }
                    Side::Sell => {
                        self.book.bids.remove(&best_price);
                    }
                }
            }
        }
    }

    pub fn depth_snapshot(&self) -> DepthSnapshot {
        DepthSnapshot::from_book(&self.book)
    }

    pub fn drain_trades(&mut self) -> Vec<Trade> {
        std::mem::take(&mut self.trades)
    }

    pub fn cancel_order(&mut self, order_id: u64) -> Result<u64, CancelError> {
        let (price, side) = self
            .book
            .index
            .remove(&order_id)
            .ok_or(CancelError::OrderNotFound)?;

        let book_side = match side {
            Side::Buy => &mut self.book.bids,
            Side::Sell => &mut self.book.asks,
        };

        let level = book_side
            .get_mut(&price)
            .expect("index pointed at non existing level");

        let pos = level
            .orders
            .iter()
            .position(|o| o.id == order_id)
            .expect("index pointed at level missing the order");

        let canceled = level.orders.remove(pos).unwrap();

        level.reduce_qty(canceled.qty);

        if level.orders.is_empty() {
            book_side.remove(&price);
        };

        Ok(canceled.qty)
    }

    pub fn amend_order_qty(&mut self, order_id: u64, new_qty: u64) -> Result<(), AmendError> {
        if new_qty == 0 {
            return Err(AmendError::InvalidQty);
        }

        let (price, side) = *self
            .book
            .index
            .get(&order_id)
            .ok_or(AmendError::OrderNotFound)?;

        let book_side = match side {
            Side::Buy => &mut self.book.bids,
            Side::Sell => &mut self.book.asks,
        };

        let level = book_side
            .get_mut(&price)
            .expect("index pointed at non-existent level");

        let order = level
            .orders
            .iter_mut()
            .find(|o| o.id == order_id)
            .expect("index pointed at level missing the order");

        if new_qty > order.qty {
            return Err(AmendError::SizeIncrease);
        }

        let delta = order.qty - new_qty;
        order.qty = new_qty;
        level.reduce_qty(delta);

        Ok(())
    }

    pub fn set_risk_gate(&mut self, risk: RiskGate) {
        self.risk = risk;
    }

    fn check_risk(&self, price: Option<u64>, qty: u64) -> Result<(), RiskRejection> {
        if let Some(max_qty) = self.risk.max_order_qty
            && qty > max_qty
        {
            return Err(RiskRejection::QtyExceedsLimit);
        }

        if let (Some(p), Some(max_notional)) = (price, self.risk.max_notional) {
            let notional = p.saturating_mul(qty);
            if notional > max_notional {
                return Err(RiskRejection::NotionalExceedsLimit);
            }
        }

        if let (Some(p), Some(max_dev), Some(mid)) =
            (price, self.risk.max_price_deviation, self.mid_price())
            && p.abs_diff(mid) > max_dev
        {
            return Err(RiskRejection::PriceTooFarFromMid);
        }

        Ok(())
    }

    fn mid_price(&self) -> Option<u64> {
        match (self.book.best_bid_price(), self.book.best_ask_price()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{MatchingEngine, Orderbook, Side};

    #[test]
    fn new_engine_has_clean_state() {
        let book = Orderbook::new();
        let engine = MatchingEngine::new(book);

        assert!(engine.trades.is_empty());
        assert!(engine.book.bids.is_empty());
        assert!(engine.book.asks.is_empty());
    }

    #[test]
    fn place_limit_order_returns_monotonically_increasing_ids() {
        let book = Orderbook::new();
        let mut engine = MatchingEngine::new(book);

        let id1 = engine.place_limit_order(100, 1, Side::Buy).unwrap();
        let id2 = engine.place_limit_order(101, 1, Side::Buy).unwrap();
        let id3 = engine.place_limit_order(102, 1, Side::Buy).unwrap();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }
}
