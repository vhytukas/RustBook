use crate::Side;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Trade {
    pub maker_id: u64,
    pub taker_id: u64,
    pub taker_side: Side,
    pub price: u64,
    pub qty: u64,
    pub timestamp: u128,
}

impl Trade {
    pub fn new(maker_id: u64, taker_id: u64, taker_side: Side, price: u64, qty: u64) -> Trade {
        let now = crate::utils::now_nanos();

        Trade {
            maker_id,
            taker_id,
            taker_side,
            price,
            qty,
            timestamp: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Side, Trade};

    #[test]
    fn new_sets_all_fields_correctly() {
        let trade = Trade::new(1, 2, Side::Buy, 100, 10);

        assert_eq!(trade.maker_id, 1);
        assert_eq!(trade.taker_id, 2);
        assert_eq!(trade.taker_side, Side::Buy);
        assert_eq!(trade.price, 100);
        assert_eq!(trade.qty, 10);
    }

    #[test]
    fn new_sets_a_nonzero_timestamp() {
        let trade = Trade::new(1, 2, Side::Buy, 100, 10);

        assert_ne!(trade.timestamp, 0);
    }
}
