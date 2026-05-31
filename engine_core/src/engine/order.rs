use super::side::Side;
use serde::*;

#[derive(Serialize, Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub price: u64,
    pub qty: u64,
    pub side: Side,
    pub timestamp: u128,
}

impl Order {
    pub fn new(id: u64, price: u64, qty: u64, side: Side) -> Order {
        let now = crate::utils::now_nanos();

        Order {
            id,
            price,
            qty,
            side,
            timestamp: now,
        }
    }

    pub fn is_filled(&self) -> bool {
        self.qty == 0
    }
}
