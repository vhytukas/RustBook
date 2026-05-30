import { useCallback, useEffect, useRef, useState } from "react";
import { WasmEngine, WasmSide } from "engine_wasm";
import { Toaster, toast } from "sonner";
import "./App.css";
import DepthPanel from "./components/DepthPanel";
import MarketMakerPanel from "./components/MarketMakerPanel";
import MetricsPanel from "./components/MetricsPanel";
import OpenOrdersPanel from "./components/OpenOrdersPanel";
import OrderEntryPanel from "./components/OrderEntryPanel";
import ReplayPanel from "./components/ReplayPanel";
import SimulationPanel from "./components/SimulationPanel";
import TopBar from "./components/TopBar";
import TradesPanel from "./components/TradesPanel";

const PRICE_SCALE = Number(WasmEngine.price_scale());

const TABS = [
  {
    id: "orders",
    label: "Open Orders",
    stage: "Stage 6",
    Panel: OpenOrdersPanel,
  },
  {
    id: "simulation",
    label: "Simulation",
    stage: "Stage 3",
    Panel: SimulationPanel,
  },
  { id: "replay", label: "Replay", stage: "Stage 4", Panel: ReplayPanel },
  { id: "metrics", label: "Metrics", stage: "Stage 5", Panel: MetricsPanel },
  { id: "mm", label: "Market Maker", stage: "Tier 2", Panel: MarketMakerPanel },
];

function App() {
  const engineRef = useRef(null);
  const [wasmReady, setWasmReady] = useState(false);
  const [wasmError, setWasmError] = useState("");
  const [depth, setDepth] = useState({ bids: [], asks: [] });
  const [tradesList, setTradesList] = useState([]);
  const [activeTab, setActiveTab] = useState("orders");

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
        setTradesList((prev) => [...formatted, ...prev]);
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

  const handleLogEngine = () => {
    if (!engineRef.current) return;

    try {
      console.log("orderbook:", engineRef.current.orderbook_full_state?.());
      console.log("WASM trades count:", engineRef.current.trades());
    } catch (error) {
      console.error("WASM call failed:", error);
    }
  };

  const handleAddTestTrade = () => {
    if (!engineRef.current) {
      console.warn("WASM engine is not ready yet");
      toast.error("Engine is not ready yet");
      return;
    }

    try {
      engineRef.current.place_limit_order(10125n, 2n, WasmSide.Buy);
      console.log("Placed test order");
      refreshSnapshot();
    } catch (error) {
      console.error("WASM call failed:", error);
    }
  };

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
        console.log("Placed market order:", { qty, side, filled });
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
      engineRef.current.place_limit_order(scaledPrice, BigInt(qty), wasmSide);
      console.log("Placed limit order:", { price, qty, side, scaledPrice });
      refreshSnapshot();
      toast.success(`Limit ${sideLabel} ${qty} @ ${price}`);
    } catch (error) {
      console.error("WASM call failed:", error);
      toast.error("Failed to place order");
    }
  };

  const statusText = wasmError
    ? `WASM error: ${wasmError}`
    : wasmReady
      ? "WASM ready"
      : "WASM loading";

  return (
    <main className="screen">
      <Toaster position="top-right" theme="dark" richColors />
      <TopBar statusText={statusText} />

      <section className="grid">
        <OrderEntryPanel
          onLogEngine={handleLogEngine}
          onAddTestTrade={handleAddTestTrade}
          onPlaceOrder={handlePlaceOrder}
        />
        <DepthPanel bids={depth.bids} asks={depth.asks} />
        <TradesPanel trades={tradesList} />
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
              <span className="tab-stage">{tab.stage}</span>
            </button>
          ))}
        </nav>

        {TABS.map(({ id, Panel }) =>
          id === activeTab ? <Panel key={id} /> : null,
        )}
      </section>
    </main>
  );
}

export default App;
