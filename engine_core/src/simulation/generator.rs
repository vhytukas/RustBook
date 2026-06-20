use super::config::{SimConfig, SimEvent, SimOrder, SimOrderKind};
use crate::Side;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct Simulator {
    rng: ChaCha8Rng,
    config: SimConfig,
}

impl Simulator {
    pub fn new(config: SimConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed);

        Self { rng, config }
    }

    // `next` here is the natural name for "produce the next event" — not the
    // Iterator trait. Simulator is an infinite stream (never terminates), so
    // implementing Iterator would force every caller to unwrap a Some forever.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> SimEvent {
        // uniform random variable
        let u: f64 = self.rng.gen_range(f64::MIN_POSITIVE..1.0);

        // inverse transform sampling - formula for getting exponential distribution from uniform numbers
        // https://en.wikipedia.org/wiki/Exponential_distribution#Random_variate_generation
        let dt_nanos = (-u.ln() / self.config.lambda_per_sec * 1e9) as u64;

        let side = if self.rng.gen_bool(0.5) {
            Side::Buy
        } else {
            Side::Sell
        };

        let is_market = self.rng.gen_bool(self.config.market_order_prob);

        let qty = self
            .rng
            .gen_range(self.config.min_qty..=self.config.max_qty);

        let kind = if is_market {
            SimOrderKind::Market
        } else {
            let spread = self.config.price_spread as i64;
            let offset = self.rng.gen_range(-spread..=spread);
            let price = (self.config.mid_price as i64 + offset).max(1) as u64;
            SimOrderKind::Limit { price }
        };

        SimEvent {
            dt_nanos,
            order: SimOrder { side, kind, qty },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(seed: u64) -> SimConfig {
        SimConfig {
            seed,
            mid_price: 100,
            price_spread: 10,
            min_qty: 1,
            max_qty: 100,
            market_order_prob: 0.1,
            lambda_per_sec: 1000.0,
        }
    }

    #[test]
    fn same_seed_produces_same_sequence() {
        let mut a = Simulator::new(test_config(42));
        let mut b = Simulator::new(test_config(42));

        for _ in 0..1000 {
            let ea = a.next();
            let eb = b.next();

            assert_eq!(ea.dt_nanos, eb.dt_nanos);
            assert_eq!(ea.order.side, eb.order.side);
            assert_eq!(ea.order.qty, eb.order.qty);

            match (&ea.order.kind, &eb.order.kind) {
                (SimOrderKind::Market, SimOrderKind::Market) => {}
                (SimOrderKind::Limit { price: p1 }, SimOrderKind::Limit { price: p2 }) => {
                    assert_eq!(p1, p2);
                }
                _ => panic!("order kinds diverged between identical seeds"),
            }
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Simulator::new(test_config(1));
        let mut b = Simulator::new(test_config(2));

        let mut diffs = 0;
        for _ in 0..100 {
            if a.next().dt_nanos != b.next().dt_nanos {
                diffs += 1;
            }
        }

        assert!(diffs > 50, "expected >50/100 dt_nanos to differ, got {diffs}");
    }

    #[test]
    fn market_order_probability_approximately_matches_config() {
        let mut sim = Simulator::new(test_config(7));
        let n = 10_000;

        let markets = (0..n)
            .filter(|_| matches!(sim.next().order.kind, SimOrderKind::Market))
            .count();

        let observed = markets as f64 / n as f64;
        let expected = 0.1;

        assert!(
            (observed - expected).abs() < 0.02,
            "observed market ratio {observed}, expected ~{expected}"
        );
    }

    #[test]
    fn dt_mean_approximately_matches_lambda() {
        let mut sim = Simulator::new(test_config(11));
        let n = 10_000u64;

        let sum: u128 = (0..n).map(|_| sim.next().dt_nanos as u128).sum();
        let mean_secs = (sum as f64 / n as f64) / 1e9;
        let expected = 1.0 / 1000.0;

        let rel_err = (mean_secs - expected).abs() / expected;
        assert!(
            rel_err < 0.05,
            "mean dt {mean_secs}s, expected ~{expected}s (rel_err {rel_err})"
        );
    }

    #[test]
    fn limit_prices_stay_within_spread() {
        let mut cfg = test_config(13);
        cfg.market_order_prob = 0.0;
        let mid = cfg.mid_price;
        let spread = cfg.price_spread;

        let mut sim = Simulator::new(cfg);

        for _ in 0..1000 {
            let ev = sim.next();
            match ev.order.kind {
                SimOrderKind::Limit { price } => {
                    assert!(
                        price >= mid - spread && price <= mid + spread,
                        "price {price} outside [{}, {}]",
                        mid - spread,
                        mid + spread
                    );
                }
                SimOrderKind::Market => panic!("market_order_prob=0 should never emit Market"),
            }
        }
    }
}
