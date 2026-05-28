use crate::{
    Orderbook,
    price_level::{self, PriceLevel},
};

use serde::*;

#[derive(Serialize)]
pub struct DepthLevel {
    price: u64,
    total_qty: u64,
}
#[derive(Serialize)]
pub struct DepthSnapshot {
    bids: Vec<DepthLevel>,
    asks: Vec<DepthLevel>,
}

impl DepthLevel {
    pub fn new(price: u64, level: &PriceLevel) -> DepthLevel {
        DepthLevel {
            price,
            total_qty: level.total_qty,
        }
    }
}

impl DepthSnapshot {
    pub fn from_book(book: &Orderbook) -> DepthSnapshot {
        let bids = book
            .bids
            .iter()
            .rev()
            .map(|(price, level)| DepthLevel::new(*price, level))
            .collect();

        let asks = book
            .asks
            .iter() // ascending = best (lowest) ask first
            .map(|(price, level)| DepthLevel::new(*price, level))
            .collect();

        DepthSnapshot { bids, asks }
    }
}

#[cfg(test)]
mod tests {
    use crate::{DepthSnapshot, Order, Orderbook, Side};

    #[test]
    fn from_empty_book_returns_empty_snapshot() {
        let book = Orderbook::new();

        let snapshot = DepthSnapshot::from_book(&book);

        assert!(snapshot.asks.is_empty());
        assert!(snapshot.bids.is_empty());
    }

    #[test]
    fn from_book_bids_reversed_and_asks_ascending() {
        let mut book = Orderbook::new();
        let bid_order1 = Order::new(1, 100, 1, Side::Buy);
        let bid_order2 = Order::new(2, 99, 1, Side::Buy);
        let bid_order3 = Order::new(3, 101, 1, Side::Buy);

        let ask_order1 = Order::new(1, 103, 1, Side::Sell);
        let ask_order2 = Order::new(2, 102, 1, Side::Sell);
        let ask_order3 = Order::new(3, 104, 1, Side::Sell);

        book.insert_order(bid_order1);
        book.insert_order(bid_order2);
        book.insert_order(bid_order3);

        book.insert_order(ask_order1);
        book.insert_order(ask_order2);
        book.insert_order(ask_order3);

        let snapshot = DepthSnapshot::from_book(&book);

        assert_eq!(snapshot.bids[0].price, 101);
        assert_eq!(snapshot.asks[0].price, 102);
    }
}
