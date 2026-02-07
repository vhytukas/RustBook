use crate::{order::Order, price_level::PriceLevel, side::Side};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Orderbook {
    pub bids: BTreeMap<u64, PriceLevel>,
    pub asks: BTreeMap<u64, PriceLevel>,
}

impl Orderbook {
    pub fn new() -> Orderbook {
        Orderbook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }
    pub fn insert_order(&mut self, order: Order) {
        let book_side = match order.side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        book_side
            .entry(order.price)
            .or_insert_with(PriceLevel::new)
            .orders
            .push_back(order);
    }

    pub fn best_bid_level(&mut self) -> Option<(u64, &PriceLevel)> {
        self.bids
            .iter()
            .next_back()
            .map(|(price, level)| (*price, level))
    }

    pub fn best_ask_level(&mut self) -> Option<(u64, &PriceLevel)> {
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

mod test {
    use crate::{order::Order, orderbook::Orderbook, side::Side};

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
}
