use super::depth::*;
use super::order::Order;
use super::orderbook::Orderbook;
use super::side::Side;
use super::trade::Trade;
use serde::*;

#[derive(Serialize)]
pub struct MatchingEngine {
    pub book: Orderbook,
    pub trades: Vec<Trade>,
    next_id: u64,
}

impl MatchingEngine {
    pub const PRICE_SCALE: u64 = 100;

    pub fn new(book: Orderbook) -> MatchingEngine {
        MatchingEngine {
            book,
            trades: Vec::new(),
            next_id: 1,
        }
    }

    pub fn place_limit_order(&mut self, price: u64, qty: u64, side: Side) -> u64 {
        let id: u64 = self.next_id;
        self.next_id += 1;

        let order = Order::new(id, price, qty, side);

        self.match_or_insert(order);

        id
    }

    pub fn place_market_order(&mut self, qty: u64, side: Side) -> (u64, u64) {
        let id = self.next_id;

        self.next_id += 1;

        let extreme = match side {
            Side::Buy => u64::MAX,
            Side::Sell => 0,
        };

        let mut order = Order::new(id, extreme, qty, side);

        self.match_against_book(&mut order);

        let filled = qty - order.qty;

        (id, filled)
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

            let level_emptied = {
                let level = match order.side {
                    Side::Buy => self.book.asks.get_mut(&best_price).unwrap(),
                    Side::Sell => self.book.bids.get_mut(&best_price).unwrap(),
                };

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
                    }
                }

                level.orders.is_empty()
            };

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

        let id1 = engine.place_limit_order(100, 1, Side::Buy);
        let id2 = engine.place_limit_order(101, 1, Side::Buy);
        let id3 = engine.place_limit_order(102, 1, Side::Buy);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }
}
