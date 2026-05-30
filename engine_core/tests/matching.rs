mod common;

use common::assert_book_invariants;
use engine_core::{MatchingEngine, Orderbook, Side};

#[test]
fn empty_book_satisfies_invariants() {
    let engine = MatchingEngine::new(Orderbook::new());

    assert_book_invariants(&engine.book);
}

#[test]
fn unmatched_buy_rests_on_book() {
    let mut engine = MatchingEngine::new(Orderbook::new());

    engine.place_limit_order(100, 5, Side::Buy);

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

    engine.place_limit_order(110, 3, Side::Sell);
    engine.place_limit_order(100, 5, Side::Buy);

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

    let maker_id = engine.place_limit_order(100, 5, Side::Sell);
    let taker_id = engine.place_limit_order(100, 5, Side::Buy);

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

    let maker_id = engine.place_limit_order(100, 3, Side::Sell);
    let taker_id = engine.place_limit_order(105, 5, Side::Buy);

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

    let maker_a = engine.place_limit_order(102, 2, Side::Buy);
    let maker_b = engine.place_limit_order(101, 3, Side::Buy);
    let maker_c = engine.place_limit_order(100, 4, Side::Buy);
    let taker_id = engine.place_limit_order(100, 7, Side::Sell);

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

    let maker_a = engine.place_limit_order(100, 3, Side::Buy);
    let maker_b = engine.place_limit_order(100, 4, Side::Buy);
    let taker_id = engine.place_limit_order(100, 3, Side::Sell);

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

    let maker_a = engine.place_limit_order(100, 2, Side::Sell);
    let maker_b = engine.place_limit_order(101, 3, Side::Sell);
    let maker_c = engine.place_limit_order(102, 4, Side::Sell);
    let taker_id = engine.place_limit_order(102, 7, Side::Buy);

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

    let maker_id = engine.place_limit_order(100, 5, Side::Sell);
    let taker_id = engine.place_limit_order(100, 3, Side::Buy);

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

    let (id, filled) = engine.place_market_order(10, Side::Buy);

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

    let maker_id = engine.place_limit_order(100, 3, Side::Sell);
    let (taker_id, filled) = engine.place_market_order(5, Side::Buy);

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

    let maker_a = engine.place_limit_order(100, 2, Side::Sell);
    let maker_b = engine.place_limit_order(101, 3, Side::Sell);
    let maker_c = engine.place_limit_order(102, 4, Side::Sell);

    let (taker_id, filled) = engine.place_market_order(7, Side::Buy);

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
