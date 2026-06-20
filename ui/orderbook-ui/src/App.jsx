import { useCallback, useEffect, useRef, useState } from "react";
import { WasmEngine, WasmReplayer, WasmSide } from "engine_wasm";
import * as Tooltip from "@radix-ui/react-tooltip";
import { Toaster, toast } from "sonner";
import "./App.css";
import DepthPanel from "./components/DepthPanel";
import MetricsPanel from "./components/MetricsPanel";
import OpenOrdersPanel from "./components/OpenOrdersPanel";
import OrderEntryPanel from "./components/OrderEntryPanel";
import ReplayPanel from "./components/ReplayPanel";
import RiskPanel from "./components/RiskPanel";
import SimulationPanel from "./components/SimulationPanel";
import TopBar from "./components/TopBar";
import TradesPanel from "./components/TradesPanel";

const PRICE_SCALE = Number(WasmEngine.price_scale());
const MAX_TRADES_DISPLAYED = 1000;
const RECORDINGS_STORAGE_KEY = "trading-engine-recordings";
const MAX_RECORDINGS = 50;

const DEFAULT_SIM_CONFIG = {
  seed: 42,
  mid_price: 10000,
  price_spread: 50,
  min_qty: 1,
  max_qty: 100,
  market_order_prob: 0.1,
  lambda_per_sec: 1000.0,
};

const TABS = [
  { id: "orders", label: "Open Orders", Panel: OpenOrdersPanel },
  { id: "simulation", label: "Simulation", Panel: SimulationPanel },
  { id: "replay", label: "Replay", Panel: ReplayPanel },
  { id: "metrics", label: "Metrics", Panel: MetricsPanel },
  { id: "risk", label: "Risk Gate", Panel: RiskPanel },
];

function App() {
  const engineRef = useRef(null);
  const [wasmReady, setWasmReady] = useState(false);
  const [wasmError, setWasmError] = useState("");
  const [depth, setDepth] = useState({ bids: [], asks: [] });
  const [tradesList, setTradesList] = useState([]);
  const [totalTrades, setTotalTrades] = useState(0);
  const [activeTab, setActiveTab] = useState("orders");
  const [simMetrics, setSimMetrics] = useState(null);
  const activeSimConfigRef = useRef(null);
  const [openOrders, setOpenOrders] = useState([]);
  const [recordings, setRecordings] = useState(() => {
    try {
      const raw = localStorage.getItem(RECORDINGS_STORAGE_KEY);
      return raw ? JSON.parse(raw) : [];
    } catch {
      return [];
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(RECORDINGS_STORAGE_KEY, JSON.stringify(recordings));
    } catch (err) {
      console.warn("Failed to persist recordings:", err);
    }
  }, [recordings]);

  const refreshSnapshot = useCallback(() => {
    const eng = engineRef.current;
    if (!eng) return;

    try {
      const snap = eng.orderbook_depth_state();
      const newTrades = eng.drain_trades();

      const buildSide = (rows) => {
        let cum = 0;
        return rows.map(({ price, total_qty }) => {
          const size = Number(total_qty);
          cum += size;
          return {
            price: Number(price) / PRICE_SCALE,
            size,
            total: cum,
          };
        });
      };

      setDepth({ bids: buildSide(snap.bids), asks: buildSide(snap.asks) });

      if (newTrades.length > 0) {
        const formatted = newTrades
          .map((t) => ({
            time: new Date(
              Number(t.timestamp / 1_000_000n),
            ).toLocaleTimeString(),
            side: String(t.taker_side).toUpperCase(),
            price: (Number(t.price) / PRICE_SCALE).toFixed(2),
            qty: Number(t.qty).toFixed(2),
          }))
          .reverse();
        setTradesList((prev) =>
          [...formatted, ...prev].slice(0, MAX_TRADES_DISPLAYED),
        );
        setTotalTrades((prev) => prev + newTrades.length);

        // Apply trades to open orders: reduce qty when our id is maker or taker.
        // Drop orders whose qty reaches zero (fully filled).
        setOpenOrders((prev) =>
          prev
            .map((order) => {
              let remaining = order.qty;
              for (const t of newTrades) {
                const tradeQty = Number(t.qty);
                if (
                  Number(t.maker_id) === order.id ||
                  Number(t.taker_id) === order.id
                ) {
                  remaining -= tradeQty;
                }
              }
              return { ...order, qty: remaining };
            })
            .filter((order) => order.qty > 0),
        );
      }
    } catch (err) {
      console.error("Snapshot failed:", err);
    }
  }, []);

  useEffect(() => {
    let mounted = true;

    try {
      engineRef.current = new WasmEngine();
      window.engine = engineRef.current;
      window.WasmReplayer = WasmReplayer;

      try {
        engineRef.current.start_simulation({
          seed: BigInt(DEFAULT_SIM_CONFIG.seed),
          mid_price: BigInt(DEFAULT_SIM_CONFIG.mid_price),
          price_spread: BigInt(DEFAULT_SIM_CONFIG.price_spread),
          min_qty: BigInt(DEFAULT_SIM_CONFIG.min_qty),
          max_qty: BigInt(DEFAULT_SIM_CONFIG.max_qty),
          market_order_prob: DEFAULT_SIM_CONFIG.market_order_prob,
          lambda_per_sec: DEFAULT_SIM_CONFIG.lambda_per_sec,
        });
        engineRef.current.burst(50n);
        activeSimConfigRef.current = JSON.stringify(DEFAULT_SIM_CONFIG);
      } catch (err) {
        console.warn("Auto-fill burst failed (non-fatal):", err);
      }

      if (mounted) {
        setWasmReady(true);
        refreshSnapshot();
      }
    } catch (err) {
      if (mounted) setWasmError(String(err));
    }

    return () => {
      mounted = false;
    };
  }, [refreshSnapshot]);

  const handleApplyRiskGate = useCallback((config) => {
    const eng = engineRef.current;
    if (!eng) throw new Error("WASM engine not ready");
    eng.set_risk_gate(config);
  }, []);

  const handleCancelOrder = useCallback(
    (id) => {
      const eng = engineRef.current;
      if (!eng) return;
      try {
        eng.cancel_order(BigInt(id));
        setOpenOrders((prev) => prev.filter((o) => o.id !== id));
        refreshSnapshot();
        toast.success(`Canceled order #${id}`);
      } catch (err) {
        console.error("Cancel failed:", err);
        toast.error(`Cancel failed: ${err}`);
      }
    },
    [refreshSnapshot],
  );

  const handleAmendOrder = useCallback(
    (id, newQty) => {
      const eng = engineRef.current;
      if (!eng) return;
      try {
        eng.amend_order_qty(BigInt(id), BigInt(newQty));
        setOpenOrders((prev) =>
          prev.map((o) => (o.id === id ? { ...o, qty: newQty } : o)),
        );
        refreshSnapshot();
        toast.success(`Amended order #${id} → qty ${newQty}`);
      } catch (err) {
        console.error("Amend failed:", err);
        toast.error(`Amend failed: ${err}`);
      }
    },
    [refreshSnapshot],
  );

  const handleBurst = useCallback(
    (n, config) => {
      const eng = engineRef.current;
      if (!eng) return;

      try {
        const configKey = JSON.stringify(config);
        if (activeSimConfigRef.current !== configKey) {
          eng.start_simulation({
            seed: BigInt(config.seed),
            mid_price: BigInt(config.mid_price),
            price_spread: BigInt(config.price_spread),
            min_qty: BigInt(config.min_qty),
            max_qty: BigInt(config.max_qty),
            market_order_prob: config.market_order_prob,
            lambda_per_sec: config.lambda_per_sec,
          });
          activeSimConfigRef.current = configKey;
        }

        const t0 = performance.now();
        const result = eng.burst(BigInt(n));
        const wall_ms = performance.now() - t0;

        refreshSnapshot();

        const orders_placed = Number(result.orders_placed);
        const trades_executed = Number(result.trades_executed);
        const seconds = wall_ms / 1000;

        setSimMetrics({
          orders_placed,
          trades_executed,
          wall_ms,
          orders_per_sec: seconds > 0 ? orders_placed / seconds : 0,
          trades_per_sec: seconds > 0 ? trades_executed / seconds : 0,
        });

        setRecordings((prev) =>
          [
            {
              id: crypto.randomUUID(),
              createdAt: Date.now(),
              config,
              totalEvents: n,
            },
            ...prev,
          ].slice(0, MAX_RECORDINGS),
        );

        toast.success(
          `Burst ${n}: ${trades_executed} trades in ${wall_ms.toFixed(1)} ms`,
        );
      } catch (err) {
        console.error("Burst failed:", err);
        toast.error("Burst failed");
      }
    },
    [refreshSnapshot],
  );

  const handlePlaceOrder = ({ type, price, qty, side }) => {
    if (!engineRef.current) {
      console.warn("WASM engine is not ready yet");
      return;
    }

    const wasmSide = side === "sell" ? WasmSide.Sell : WasmSide.Buy;
    const sideLabel = side.toUpperCase();

    try {
      if (type === "market") {
        const result = engineRef.current.place_market_order(
          BigInt(qty),
          wasmSide,
        );
        const filled = Number(result[1]);

        refreshSnapshot();

        if (filled === 0) {
          toast.error(`Market ${sideLabel} ${qty}: no liquidity available`);
        } else if (filled < qty) {
          toast.warning(
            `Market ${sideLabel}: partially filled ${filled} of ${qty}`,
          );
        } else {
          toast.success(`Market ${sideLabel} ${qty}: filled in full`);
        }
        return;
      }

      const scaledPrice = BigInt(Math.round(price * PRICE_SCALE));
      const id = Number(
        engineRef.current.place_limit_order(scaledPrice, BigInt(qty), wasmSide),
      );

      setOpenOrders((prev) => [
        ...prev,
        {
          id,
          price: Number(scaledPrice) / PRICE_SCALE,
          qty: Math.trunc(qty),
          originalQty: Math.trunc(qty),
          side,
          placedAt: Date.now(),
        },
      ]);

      refreshSnapshot();
      toast.success(`Limit ${sideLabel} ${qty} @ ${price}`);
    } catch (error) {
      console.error("WASM call failed:", error);
      toast.error(`Order rejected: ${error}`);
    }
  };

  return (
    <Tooltip.Provider delayDuration={150}>
      <main className="screen">
        <Toaster position="top-right" theme="dark" richColors />
        <TopBar />

        <section className="grid">
          <OrderEntryPanel onPlaceOrder={handlePlaceOrder} />
          <DepthPanel bids={depth.bids} asks={depth.asks} />
          <TradesPanel
            trades={tradesList}
            totalCount={totalTrades}
            maxDisplayed={MAX_TRADES_DISPLAYED}
          />
        </section>

        <section className="dashboard">
          <nav className="tab-nav">
            {TABS.map((tab) => (
              <button
                key={tab.id}
                type="button"
                className={`tab ${activeTab === tab.id ? "active" : ""}`}
                onClick={() => setActiveTab(tab.id)}
              >
                {tab.label}
              </button>
            ))}
          </nav>

          {TABS.map(({ id, Panel }) => {
            if (id !== activeTab) return null;
            if (id === "simulation") {
              return (
                <Panel
                  key={id}
                  onBurst={handleBurst}
                  metrics={simMetrics}
                  defaultConfig={DEFAULT_SIM_CONFIG}
                />
              );
            }
            if (id === "replay") {
              return (
                <Panel
                  key={id}
                  recordings={recordings}
                  setRecordings={setRecordings}
                />
              );
            }
            if (id === "risk") {
              return <Panel key={id} onApply={handleApplyRiskGate} />;
            }
            if (id === "orders") {
              return (
                <Panel
                  key={id}
                  orders={openOrders}
                  onCancel={handleCancelOrder}
                  onAmend={handleAmendOrder}
                />
              );
            }
            return <Panel key={id} />;
          })}
        </section>
      </main>
    </Tooltip.Provider>
  );
}

export default App;
