use super::{order::Order, price_level::PriceLevel, side::Side};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Serialize)]
pub struct Orderbook {
    pub bids: BTreeMap<u64, PriceLevel>,
    pub asks: BTreeMap<u64, PriceLevel>,
    #[serde(skip)]
    pub index: HashMap<u64, (u64, Side)>,
}

impl Default for Orderbook {
    fn default() -> Self {
        Self::new()
    }
}

impl Orderbook {
    pub fn new() -> Orderbook {
        Orderbook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            index: HashMap::new(),
        }
    }
    pub fn insert_order(&mut self, order: Order) {
        self.index.insert(order.id, (order.price, order.side));

        let book_side = match order.side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        let level = book_side.entry(order.price).or_insert_with(PriceLevel::new);
        level.add_order(order);
    }

    pub fn best_bid_level(&self) -> Option<(u64, &PriceLevel)> {
        self.bids
            .iter()
            .next_back()
            .map(|(price, level)| (*price, level))
    }

    pub fn best_ask_level(&self) -> Option<(u64, &PriceLevel)> {
        self.asks
            .iter()
            .next()
            .map(|(price, level)| (*price, level))
    }

    pub fn best_bid_price(&self) -> Option<u64> {
        self.bids.last_key_value().map(|(p, _)| *p)
    }

    pub fn best_ask_price(&self) -> Option<u64> {
        self.asks.first_key_value().map(|(p, _)| *p)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Order, Orderbook, Side};

    #[test]
    fn correctly_inserts_new_order() {
        let mut orderbook = Orderbook::new();
        let order = Order::new(0, 100, 10, Side::Buy);

        let id = order.id;
        let price = order.price;
        let qty = order.qty;
        let side = order.side;
        let timestamp = order.timestamp;

        orderbook.insert_order(order);

        let level = orderbook.bids.get(&price).unwrap();
        let pulled_order = level.orders.front().unwrap();

        assert_eq!(pulled_order.id, id);
        assert_eq!(pulled_order.price, price);
        assert_eq!(pulled_order.qty, qty);
        assert_eq!(pulled_order.side, side);
        assert_eq!(pulled_order.timestamp, timestamp);

        assert_eq!(orderbook.asks.len(), 0);
        assert_eq!(orderbook.bids.len(), 1);

        assert!(orderbook.bids.contains_key(&100));
        assert_eq!(orderbook.best_bid_price(), Some(100));
        assert_eq!(level.orders.len(), 1);
    }

    #[test]
    fn insert_appends_to_existing_level() {
        let mut orderbook = Orderbook::new();

        let old_order = Order::new(1, 100, 1, Side::Buy);
        let new_order = Order::new(2, 100, 2, Side::Buy);

        orderbook.insert_order(old_order);
        orderbook.insert_order(new_order);

        let price_level = orderbook
            .best_bid_level()
            .expect("best bid should exist after two inserts")
            .1;

        assert_eq!(orderbook.bids.len(), 1);
        assert_eq!(price_level.orders.len(), 2);
        assert_eq!(price_level.total_qty, 3);
        assert_eq!(price_level.orders.front().unwrap().id, 1);
        assert_eq!(price_level.orders.back().unwrap().id, 2);
    }

    #[test]
    fn insert_routes_buy_to_bids_and_sell_to_asks() {
        let mut orderbook = Orderbook::new();

        let buy_order = Order::new(1, 100, 1, Side::Buy);
        let sell_order = Order::new(2, 200, 1, Side::Sell);

        orderbook.insert_order(buy_order);
        orderbook.insert_order(sell_order);

        let (_, bid_price_level) = orderbook
            .best_bid_level()
            .expect("expected bid level to exist in orderbook");

        let (_, ask_price_level) = orderbook
            .best_ask_level()
            .expect("expected ask level to exist in orderbook");

        assert_eq!(orderbook.bids.len(), 1);
        assert_eq!(orderbook.asks.len(), 1);

        assert_eq!(bid_price_level.orders.front().unwrap().id, 1);
        assert_eq!(ask_price_level.orders.front().unwrap().id, 2);

        assert!(orderbook.bids.contains_key(&100));
        assert!(orderbook.asks.contains_key(&200));
        assert!(!orderbook.bids.contains_key(&200));
        assert!(!orderbook.asks.contains_key(&100));

        assert_eq!(bid_price_level.orders.front().unwrap().side, Side::Buy);
        assert_eq!(ask_price_level.orders.front().unwrap().side, Side::Sell);
    }

    #[test]
    fn best_prices_return_none_on_empty_book() {
        let orderbook = Orderbook::new();

        assert!(orderbook.best_ask_level().is_none());
        assert!(orderbook.best_bid_level().is_none());
        assert!(orderbook.best_ask_price().is_none());
        assert!(orderbook.best_bid_price().is_none());
    }

    #[test]
    fn best_bid_price_returns_highest_when_multiple_levels() {
        let mut orderbook = Orderbook::new();

        let buy_order1 = Order::new(1, 100, 1, Side::Buy);
        let buy_order2 = Order::new(2, 99, 1, Side::Buy);
        let buy_order3 = Order::new(3, 101, 1, Side::Buy);

        orderbook.insert_order(buy_order1);
        orderbook.insert_order(buy_order2);
        orderbook.insert_order(buy_order3);

        assert_eq!(orderbook.best_bid_price(), Some(101));
    }
    #[test]
    fn best_ask_price_returns_lowest_when_multiple_levels() {
        let mut orderbook = Orderbook::new();

        let sell_order1 = Order::new(1, 100, 1, Side::Sell);
        let sell_order2 = Order::new(2, 99, 1, Side::Sell);
        let sell_order3 = Order::new(3, 101, 1, Side::Sell);

        orderbook.insert_order(sell_order1);
        orderbook.insert_order(sell_order2);
        orderbook.insert_order(sell_order3);

        assert_eq!(orderbook.best_ask_price(), Some(99));
    }
}
