mod common;

use common::assert_book_invariants;
use engine_core::{
    AmendError, CancelError, MatchingEngine, Orderbook, RiskGate, RiskRejection, Side,
};

#[test]
fn empty_book_satisfies_invariants() {
    let engine = MatchingEngine::new(Orderbook::new());

    assert_book_invariants(&engine.book);
}

#[test]
fn unmatched_buy_rests_on_book() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    engine.place_limit_order(100, 5, Side::Buy).unwrap();

    assert!(engine.trades.is_empty());
    assert!(engine.book.asks.is_empty());
    assert_eq!(engine.book.bids.len(), 1);
    assert_eq!(engine.book.bids[&100].orders.len(), 1);
    assert_eq!(engine.book.bids[&100].total_qty, 5);
    assert_eq!(engine.book.best_bid_price(), Some(100));
    assert!(engine.book.best_ask_price().is_none());

    assert_book_invariants(&engine.book);
}

#[test]
fn no_cross_buy_rests_alongside_existing_ask() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    engine.place_limit_order(110, 3, Side::Sell).unwrap();
    engine.place_limit_order(100, 5, Side::Buy).unwrap();

    assert!(engine.trades.is_empty());
    assert_eq!(engine.book.bids.len(), 1);
    assert_eq!(engine.book.asks.len(), 1);
    assert_eq!(engine.book.bids[&100].orders.len(), 1);
    assert_eq!(engine.book.bids[&100].total_qty, 5);
    assert_eq!(engine.book.asks[&110].orders.len(), 1);
    assert_eq!(engine.book.asks[&110].total_qty, 3);
    assert_eq!(engine.book.best_bid_price(), Some(100));
    assert_eq!(engine.book.best_ask_price(), Some(110));

    assert_book_invariants(&engine.book);
}

#[test]
fn exact_fill_emits_one_trade_and_empties_book() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_id = engine.place_limit_order(100, 5, Side::Sell).unwrap();
    let taker_id = engine.place_limit_order(100, 5, Side::Buy).unwrap();

    assert_eq!(engine.trades.len(), 1);
    assert_eq!(engine.trades[0].maker_id, maker_id);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Buy);
    assert_eq!(engine.trades[0].price, 100);
    assert_eq!(engine.trades[0].qty, 5);

    assert!(engine.book.bids.is_empty());
    assert!(engine.book.asks.is_empty());
    assert!(engine.book.best_bid_price().is_none());
    assert!(engine.book.best_ask_price().is_none());

    assert_book_invariants(&engine.book);
}

#[test]
fn partial_fill_taker_qty_exceeds_maker() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_id = engine.place_limit_order(100, 3, Side::Sell).unwrap();
    let taker_id = engine.place_limit_order(105, 5, Side::Buy).unwrap();

    assert_eq!(engine.trades.len(), 1);
    assert_eq!(engine.trades[0].maker_id, maker_id);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Buy);
    assert_eq!(engine.trades[0].price, 100);
    assert_eq!(engine.trades[0].qty, 3);

    assert!(engine.book.asks.is_empty());
    assert_eq!(engine.book.bids.len(), 1);
    assert_eq!(engine.book.bids[&105].orders.len(), 1);
    assert_eq!(engine.book.bids[&105].total_qty, 2);
    assert_eq!(engine.book.best_bid_price(), Some(105));
    assert!(engine.book.best_ask_price().is_none());

    assert_book_invariants(&engine.book);
}

#[test]
fn sell_taker_sweeps_multiple_bid_levels_until_filled() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_a = engine.place_limit_order(102, 2, Side::Buy).unwrap();
    let maker_b = engine.place_limit_order(101, 3, Side::Buy).unwrap();
    let maker_c = engine.place_limit_order(100, 4, Side::Buy).unwrap();
    let taker_id = engine.place_limit_order(100, 7, Side::Sell).unwrap();

    assert_eq!(engine.trades.len(), 3);

    assert_eq!(engine.trades[0].maker_id, maker_a);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Sell);
    assert_eq!(engine.trades[0].price, 102);
    assert_eq!(engine.trades[0].qty, 2);

    assert_eq!(engine.trades[1].maker_id, maker_b);
    assert_eq!(engine.trades[1].taker_id, taker_id);
    assert_eq!(engine.trades[1].taker_side, Side::Sell);
    assert_eq!(engine.trades[1].price, 101);
    assert_eq!(engine.trades[1].qty, 3);

    assert_eq!(engine.trades[2].maker_id, maker_c);
    assert_eq!(engine.trades[2].taker_id, taker_id);
    assert_eq!(engine.trades[2].taker_side, Side::Sell);
    assert_eq!(engine.trades[2].price, 100);
    assert_eq!(engine.trades[2].qty, 2);

    assert!(engine.book.asks.is_empty());
    assert_eq!(engine.book.bids.len(), 1);
    assert_eq!(engine.book.bids[&100].orders.len(), 1);

    let surviving_maker = engine.book.bids[&100].orders.front().unwrap();
    assert_eq!(surviving_maker.id, maker_c);
    assert_eq!(surviving_maker.qty, 2);

    assert_eq!(engine.book.bids[&100].total_qty, 2);
    assert_eq!(engine.book.best_bid_price(), Some(100));
    assert!(engine.book.best_ask_price().is_none());

    assert_book_invariants(&engine.book);
}

#[test]
fn fifo_oldest_order_at_same_price_matches_first() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_a = engine.place_limit_order(100, 3, Side::Buy).unwrap();
    let maker_b = engine.place_limit_order(100, 4, Side::Buy).unwrap();
    let taker_id = engine.place_limit_order(100, 3, Side::Sell).unwrap();

    assert_eq!(engine.trades.len(), 1);
    assert_eq!(engine.trades[0].maker_id, maker_a);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Sell);
    assert_eq!(engine.trades[0].price, 100);
    assert_eq!(engine.trades[0].qty, 3);

    assert!(engine.book.asks.is_empty());
    assert_eq!(engine.book.bids.len(), 1);
    assert_eq!(engine.book.bids[&100].orders.len(), 1);

    let survivor = engine.book.bids[&100].orders.front().unwrap();
    assert_eq!(survivor.id, maker_b);
    assert_eq!(survivor.qty, 4);

    assert_eq!(engine.book.bids[&100].total_qty, 4);
    assert_eq!(engine.book.best_bid_price(), Some(100));
    assert!(engine.book.best_ask_price().is_none());

    assert_book_invariants(&engine.book);
}

#[test]
fn taker_sweeps_multiple_levels_until_filled() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_a = engine.place_limit_order(100, 2, Side::Sell).unwrap();
    let maker_b = engine.place_limit_order(101, 3, Side::Sell).unwrap();
    let maker_c = engine.place_limit_order(102, 4, Side::Sell).unwrap();
    let taker_id = engine.place_limit_order(102, 7, Side::Buy).unwrap();

    assert_eq!(engine.trades.len(), 3);

    assert_eq!(engine.trades[0].maker_id, maker_a);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Buy);
    assert_eq!(engine.trades[0].price, 100);
    assert_eq!(engine.trades[0].qty, 2);

    assert_eq!(engine.trades[1].maker_id, maker_b);
    assert_eq!(engine.trades[1].taker_id, taker_id);
    assert_eq!(engine.trades[1].taker_side, Side::Buy);
    assert_eq!(engine.trades[1].price, 101);
    assert_eq!(engine.trades[1].qty, 3);

    assert_eq!(engine.trades[2].maker_id, maker_c);
    assert_eq!(engine.trades[2].taker_id, taker_id);
    assert_eq!(engine.trades[2].taker_side, Side::Buy);
    assert_eq!(engine.trades[2].price, 102);
    assert_eq!(engine.trades[2].qty, 2);

    assert!(engine.book.bids.is_empty());
    assert_eq!(engine.book.asks.len(), 1);
    assert_eq!(engine.book.asks[&102].orders.len(), 1);

    let surviving_maker = engine.book.asks[&102].orders.front().unwrap();
    assert_eq!(surviving_maker.id, maker_c);
    assert_eq!(surviving_maker.qty, 2);

    assert_eq!(engine.book.asks[&102].total_qty, 2);
    assert!(engine.book.best_bid_price().is_none());
    assert_eq!(engine.book.best_ask_price(), Some(102));

    assert_book_invariants(&engine.book);
}

#[test]
fn partial_fill_maker_qty_exceeds_taker() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_id = engine.place_limit_order(100, 5, Side::Sell).unwrap();
    let taker_id = engine.place_limit_order(100, 3, Side::Buy).unwrap();

    assert_eq!(engine.trades.len(), 1);
    assert_eq!(engine.trades[0].maker_id, maker_id);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Buy);
    assert_eq!(engine.trades[0].price, 100);
    assert_eq!(engine.trades[0].qty, 3);

    assert!(engine.book.bids.is_empty());
    assert_eq!(engine.book.asks.len(), 1);
    assert_eq!(engine.book.asks[&100].orders.len(), 1);

    let surviving_maker = engine.book.asks[&100].orders.front().unwrap();
    assert_eq!(surviving_maker.id, maker_id);
    assert_eq!(surviving_maker.qty, 2);

    assert_eq!(engine.book.asks[&100].total_qty, 2);
    assert!(engine.book.best_bid_price().is_none());
    assert_eq!(engine.book.best_ask_price(), Some(100));

    assert_book_invariants(&engine.book);
}

#[test]
fn market_buy_on_empty_book_returns_zero_filled() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let (id, filled) = engine.place_market_order(10, Side::Buy).unwrap();

    assert!(engine.trades.is_empty());
    assert_eq!(filled, 0);
    assert_eq!(id, 1);
    assert!(engine.book.bids.is_empty());
    assert!(engine.book.asks.is_empty());
    assert!(engine.book.best_bid_price().is_none());
    assert!(engine.book.best_ask_price().is_none());

    assert_book_invariants(&engine.book);
}

#[test]
fn market_buy_partial_fill_drops_residual() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_id = engine.place_limit_order(100, 3, Side::Sell).unwrap();
    let (taker_id, filled) = engine.place_market_order(5, Side::Buy).unwrap();

    assert_eq!(engine.trades.len(), 1);
    assert_eq!(engine.trades[0].maker_id, maker_id);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Buy);
    assert_eq!(engine.trades[0].price, 100);
    assert_eq!(engine.trades[0].qty, 3);

    assert_eq!(filled, 3);

    assert!(engine.book.bids.is_empty());
    assert!(engine.book.asks.is_empty());
    assert!(engine.book.best_bid_price().is_none());
    assert!(engine.book.best_ask_price().is_none());

    assert_book_invariants(&engine.book);
}

#[test]
fn market_buy_sweeps_multiple_levels() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_a = engine.place_limit_order(100, 2, Side::Sell).unwrap();
    let maker_b = engine.place_limit_order(101, 3, Side::Sell).unwrap();
    let maker_c = engine.place_limit_order(102, 4, Side::Sell).unwrap();

    let (taker_id, filled) = engine.place_market_order(7, Side::Buy).unwrap();

    assert_eq!(engine.trades.len(), 3);

    assert_eq!(engine.trades[0].maker_id, maker_a);
    assert_eq!(engine.trades[0].taker_id, taker_id);
    assert_eq!(engine.trades[0].taker_side, Side::Buy);
    assert_eq!(engine.trades[0].price, 100);
    assert_eq!(engine.trades[0].qty, 2);

    assert_eq!(engine.trades[1].maker_id, maker_b);
    assert_eq!(engine.trades[1].taker_id, taker_id);
    assert_eq!(engine.trades[1].taker_side, Side::Buy);
    assert_eq!(engine.trades[1].price, 101);
    assert_eq!(engine.trades[1].qty, 3);

    assert_eq!(engine.trades[2].maker_id, maker_c);
    assert_eq!(engine.trades[2].taker_id, taker_id);
    assert_eq!(engine.trades[2].taker_side, Side::Buy);
    assert_eq!(engine.trades[2].price, 102);
    assert_eq!(engine.trades[2].qty, 2);

    assert_eq!(filled, 7);

    assert!(engine.book.bids.is_empty());
    assert_eq!(engine.book.asks.len(), 1);
    assert_eq!(engine.book.asks[&102].orders.len(), 1);

    let surviving_maker = engine.book.asks[&102].orders.front().unwrap();
    assert_eq!(surviving_maker.id, maker_c);
    assert_eq!(surviving_maker.qty, 2);

    assert_eq!(engine.book.asks[&102].total_qty, 2);
    assert!(engine.book.best_bid_price().is_none());
    assert_eq!(engine.book.best_ask_price(), Some(102));

    assert_book_invariants(&engine.book);
}

#[test]
fn index_populated_on_insert() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let id_a = engine.place_limit_order(100, 5, Side::Buy).unwrap();
    let id_b = engine.place_limit_order(101, 3, Side::Buy).unwrap();
    let id_c = engine.place_limit_order(200, 7, Side::Sell).unwrap();

    assert_eq!(engine.book.index.len(), 3);
    assert_eq!(engine.book.index.get(&id_a), Some(&(100, Side::Buy)));
    assert_eq!(engine.book.index.get(&id_b), Some(&(101, Side::Buy)));
    assert_eq!(engine.book.index.get(&id_c), Some(&(200, Side::Sell)));

    assert_book_invariants(&engine.book);
}

#[test]
fn index_cleared_on_full_fill() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_id = engine.place_limit_order(100, 5, Side::Sell).unwrap();
    assert!(engine.book.index.contains_key(&maker_id));

    // Taker fully consumes the maker
    engine.place_limit_order(100, 5, Side::Buy).unwrap();

    assert!(
        !engine.book.index.contains_key(&maker_id),
        "fully-filled maker should be removed from index"
    );
    assert!(engine.book.index.is_empty());

    assert_book_invariants(&engine.book);
}

#[test]
fn index_persists_through_partial_fill() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    let maker_id = engine.place_limit_order(100, 10, Side::Sell).unwrap();

    // Taker only consumes part of the maker
    engine.place_limit_order(100, 3, Side::Buy).unwrap();

    // Maker still rests with qty=7, should still be in index
    assert_eq!(
        engine.book.index.get(&maker_id),
        Some(&(100, Side::Sell)),
        "partially-filled maker should remain in index"
    );
    assert_eq!(engine.book.asks[&100].orders.front().unwrap().qty, 7);

    assert_book_invariants(&engine.book);
}

#[test]
fn index_empty_after_full_sweep() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    // Three resting asks at distinct prices
    engine.place_limit_order(100, 1, Side::Sell).unwrap();
    engine.place_limit_order(101, 1, Side::Sell).unwrap();
    engine.place_limit_order(102, 1, Side::Sell).unwrap();
    assert_eq!(engine.book.index.len(), 3);

    // Market buy sweeps all of them
    let (_, filled) = engine.place_market_order(3, Side::Buy).unwrap();
    assert_eq!(filled, 3);

    assert!(
        engine.book.index.is_empty(),
        "all makers swept, index should be empty"
    );
    assert!(engine.book.asks.is_empty());

    assert_book_invariants(&engine.book);
}

#[test]
fn cancel_removes_resting_order() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id = engine.place_limit_order(100, 5, Side::Buy).unwrap();

    let canceled_qty = engine.cancel_order(id).unwrap();

    assert_eq!(canceled_qty, 5);
    assert!(engine.book.bids.is_empty());
    assert!(engine.book.index.is_empty());
    assert_book_invariants(&engine.book);
}

#[test]
fn cancel_nonexistent_returns_err() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    engine.place_limit_order(100, 5, Side::Buy).unwrap();

    let err = engine.cancel_order(9999).unwrap_err();
    assert_eq!(err, CancelError::OrderNotFound);

    // Existing order untouched
    assert_eq!(engine.book.bids[&100].orders.len(), 1);
    assert_book_invariants(&engine.book);
}

#[test]
fn cancel_after_partial_fill_returns_remaining_qty() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let maker_id = engine.place_limit_order(100, 10, Side::Sell).unwrap();
    engine.place_limit_order(100, 3, Side::Buy).unwrap(); // partial fill, maker has 7 left

    let canceled_qty = engine.cancel_order(maker_id).unwrap();

    assert_eq!(canceled_qty, 7);
    assert!(engine.book.asks.is_empty());
    assert!(engine.book.index.is_empty());
    assert_book_invariants(&engine.book);
}

#[test]
fn cancel_preserves_fifo_at_level() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id_a = engine.place_limit_order(100, 1, Side::Buy).unwrap();
    let id_b = engine.place_limit_order(100, 1, Side::Buy).unwrap();
    let id_c = engine.place_limit_order(100, 1, Side::Buy).unwrap();

    engine.cancel_order(id_b).unwrap();

    let remaining: Vec<u64> = engine.book.bids[&100]
        .orders
        .iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(remaining, vec![id_a, id_c]);
    assert_book_invariants(&engine.book);
}

#[test]
fn cancel_empties_level_removes_it_from_book() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id = engine.place_limit_order(100, 5, Side::Buy).unwrap();
    engine.place_limit_order(101, 5, Side::Buy).unwrap();

    engine.cancel_order(id).unwrap();

    assert!(!engine.book.bids.contains_key(&100));
    assert!(engine.book.bids.contains_key(&101));
    assert_book_invariants(&engine.book);
}

#[test]
fn cancel_twice_returns_err_second_time() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id = engine.place_limit_order(100, 5, Side::Buy).unwrap();

    engine.cancel_order(id).unwrap();
    let err = engine.cancel_order(id).unwrap_err();
    assert_eq!(err, CancelError::OrderNotFound);
}

#[test]
fn amend_down_preserves_fifo_position() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id_a = engine.place_limit_order(100, 5, Side::Buy).unwrap();
    let id_b = engine.place_limit_order(100, 5, Side::Buy).unwrap();
    let id_c = engine.place_limit_order(100, 5, Side::Buy).unwrap();

    engine.amend_order_qty(id_b, 2).unwrap();

    // Order still in the same position — front-to-back is still A, B, C
    let positions: Vec<u64> = engine.book.bids[&100]
        .orders
        .iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(positions, vec![id_a, id_b, id_c]);

    // B's qty was reduced
    let b = engine.book.bids[&100]
        .orders
        .iter()
        .find(|o| o.id == id_b)
        .unwrap();
    assert_eq!(b.qty, 2);

    assert_book_invariants(&engine.book);
}

#[test]
fn amend_down_updates_level_total_qty() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id = engine.place_limit_order(100, 10, Side::Buy).unwrap();
    assert_eq!(engine.book.bids[&100].total_qty, 10);

    engine.amend_order_qty(id, 3).unwrap();

    assert_eq!(engine.book.bids[&100].total_qty, 3);
    assert_book_invariants(&engine.book);
}

#[test]
fn amend_up_returns_err() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id = engine.place_limit_order(100, 5, Side::Buy).unwrap();

    let err = engine.amend_order_qty(id, 10).unwrap_err();
    assert_eq!(err, AmendError::SizeIncrease);

    // Original order untouched
    assert_eq!(engine.book.bids[&100].orders.front().unwrap().qty, 5);
    assert_eq!(engine.book.bids[&100].total_qty, 5);
    assert_book_invariants(&engine.book);
}

#[test]
fn amend_to_zero_returns_err() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id = engine.place_limit_order(100, 5, Side::Buy).unwrap();

    let err = engine.amend_order_qty(id, 0).unwrap_err();
    assert_eq!(err, AmendError::InvalidQty);

    // Order still there, untouched
    assert_eq!(engine.book.bids[&100].orders.front().unwrap().qty, 5);
    assert_book_invariants(&engine.book);
}

#[test]
fn amend_nonexistent_returns_err() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    engine.place_limit_order(100, 5, Side::Buy).unwrap();

    let err = engine.amend_order_qty(9999, 2).unwrap_err();
    assert_eq!(err, AmendError::OrderNotFound);
    assert_book_invariants(&engine.book);
}

#[test]
fn amend_does_not_change_index() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    let id = engine.place_limit_order(100, 10, Side::Buy).unwrap();

    let index_entry_before = *engine.book.index.get(&id).unwrap();
    engine.amend_order_qty(id, 3).unwrap();
    let index_entry_after = *engine.book.index.get(&id).unwrap();

    // Index entry must be untouched — amend doesn't change (price, side)
    assert_eq!(index_entry_before, index_entry_after);
    assert_eq!(engine.book.index.len(), 1);

    assert_book_invariants(&engine.book);
}

#[test]
fn risk_qty_limit_rejects_oversized_order() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    engine.set_risk_gate(RiskGate {
        max_order_qty: Some(100),
        ..RiskGate::default()
    });

    let err = engine.place_limit_order(100, 200, Side::Buy).unwrap_err();
    assert_eq!(err, RiskRejection::QtyExceedsLimit);

    assert!(engine.book.bids.is_empty());
    assert!(engine.book.index.is_empty());
}

#[test]
fn risk_qty_limit_allows_at_boundary() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    engine.set_risk_gate(RiskGate {
        max_order_qty: Some(100),
        ..RiskGate::default()
    });

    engine.place_limit_order(100, 100, Side::Buy).unwrap();
    assert_eq!(engine.book.bids[&100].total_qty, 100);
}

#[test]
fn risk_notional_limit_rejects() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    engine.set_risk_gate(RiskGate {
        max_notional: Some(10_000),
        ..RiskGate::default()
    });

    // 100 × 200 = 20,000, over the 10,000 cap
    let err = engine.place_limit_order(100, 200, Side::Buy).unwrap_err();
    assert_eq!(err, RiskRejection::NotionalExceedsLimit);
}

#[test]
fn risk_price_deviation_rejects_when_far_from_mid() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    // Build a book with mid = 100
    engine.place_limit_order(99, 1, Side::Buy).unwrap();
    engine.place_limit_order(101, 1, Side::Sell).unwrap();

    engine.set_risk_gate(RiskGate {
        max_price_deviation: Some(10),
        ..RiskGate::default()
    });

    // 200 is way outside mid ± 10
    let err = engine.place_limit_order(200, 1, Side::Buy).unwrap_err();
    assert_eq!(err, RiskRejection::PriceTooFarFromMid);
}

#[test]
fn risk_price_deviation_skipped_when_no_mid() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    engine.set_risk_gate(RiskGate {
        max_price_deviation: Some(10),
        ..RiskGate::default()
    });

    // Empty book → no mid → check should silently pass
    engine.place_limit_order(200, 1, Side::Buy).unwrap();
    assert_eq!(engine.book.bids[&200].total_qty, 1);
}

#[test]
fn risk_market_order_only_checks_qty() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    engine.set_risk_gate(RiskGate {
        max_order_qty: Some(50),
        max_notional: Some(1),
        max_price_deviation: Some(1),
    });

    // Under qty cap — notional + deviation don't apply to market orders
    let (_, filled) = engine.place_market_order(10, Side::Buy).unwrap();
    assert_eq!(filled, 0); // empty book

    // Over qty cap — rejected
    let err = engine.place_market_order(100, Side::Buy).unwrap_err();
    assert_eq!(err, RiskRejection::QtyExceedsLimit);
}

#[test]
fn no_risk_gate_allows_anything() {
    let mut engine = MatchingEngine::new(Orderbook::new());
    // No set_risk_gate call → defaults to all None → no checks

    engine
        .place_limit_order(u64::MAX / 2, u64::MAX / 2, Side::Buy)
        .unwrap();
}
