use crate::order::Order;
use crate::orderbook::Orderbook;
use crate::side::Side;
use crate::trade::Trade;

pub struct MatchingEngine {
    pub book: Orderbook,
    pub trades: Vec<Trade>,
    next_id: u64,
}

impl MatchingEngine {
    pub fn new(book: Orderbook) -> MatchingEngine {
        MatchingEngine {
            book,
            trades: Vec::new(),
            next_id: 1,
        }
    }

    pub fn place_limit_order(&mut self, price: u64, qty: u64, side: Side) -> u64 {
        let id: u64 = self.next_id;

        let order = Order::new(id, price, qty, side);
        self.next_id += 1;

        self.match_or_insert(order);

        id
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
                    let Some(front) = level.orders.front_mut() else {
                        break;
                    };

                    let maker_id = front.id;
                    let fill_qty = order.qty.min(front.qty);

                    self.trades
                        .push(Trade::new(maker_id, order.id, best_price, fill_qty));

                    order.qty -= fill_qty;
                    front.qty -= fill_qty;

                    if front.qty == 0 {
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
}
