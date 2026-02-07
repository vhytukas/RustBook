pub struct Trade {
    pub maker_id: u64,
    pub taker_id: u64,
    pub price: u64,
    pub qty: u64,
    pub timestamp: u128,
}

impl Trade {
    pub fn new(maker_id: u64, taker_id: u64, price: u64, qty: u64) -> Trade {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        Trade {
            maker_id,
            taker_id,
            price,
            qty,
            timestamp: now,
        }
    }
}
