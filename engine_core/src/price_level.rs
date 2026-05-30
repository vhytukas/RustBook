use crate::order::Order;
use serde::*;
use std::collections::VecDeque;

#[derive(Serialize, Debug)]
pub struct PriceLevel {
    pub orders: VecDeque<Order>,
    pub total_qty: u64,
}

impl PriceLevel {
    pub fn new() -> PriceLevel {
        PriceLevel {
            orders: VecDeque::new(),
            total_qty: 0,
        }
    }

    pub fn add_order(&mut self, order: Order) {
        self.total_qty = self
            .total_qty
            .checked_add(order.qty)
            .expect("total_qty overflow at price level");
        self.orders.push_back(order);
    }

    pub fn reduce_qty(&mut self, filled: u64) {
        self.total_qty = self.total_qty.saturating_sub(filled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Side;

    #[test]
    fn new_level_is_empty() {
        let price_level = PriceLevel::new();

        assert!(price_level.orders.is_empty());
        assert_eq!(price_level.total_qty, 0);
    }

    #[test]
    fn add_order_pushes_back_and_increments_total() {
        let mut price_level = PriceLevel::new();

        let order1 = Order::new(1, 100, 2, Side::Buy);
        let order2 = Order::new(2, 100, 5, Side::Buy);

        let total_qty = order1.qty + order2.qty;

        price_level.add_order(order1);
        price_level.add_order(order2);

        assert_eq!(price_level.total_qty, total_qty);
        assert_eq!(price_level.orders.front().unwrap().id, 1);
        assert_eq!(price_level.orders.back().unwrap().id, 2);
    }

    #[test]
    fn reduce_qty_decrements_total() {
        let mut price_level = PriceLevel::new();
        let order = Order::new(1, 100, 5, Side::Buy);

        price_level.add_order(order);
        price_level.reduce_qty(3);

        assert_eq!(price_level.total_qty, 2);
    }

    #[test]
    fn reduce_qty_saturates_at_zero() {
        let mut price_level = PriceLevel::new();
        let order = Order::new(1, 100, 5, Side::Buy);

        price_level.add_order(order);
        price_level.reduce_qty(u64::MAX);

        assert_eq!(price_level.total_qty, 0);
    }
}
