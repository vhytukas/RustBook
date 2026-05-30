import { useState } from "react";

const PRICE_MAX = 200;
const QTY_MAX = 100;

const OrderEntryPanel = ({ onLogEngine, onAddTestTrade, onPlaceOrder }) => {
  const [side, setSide] = useState("buy");
  const [orderType, setOrderType] = useState("limit");
  const [price, setPrice] = useState("");
  const [qty, setQty] = useState("");

  const isMarket = orderType === "market";
  const hasEmptyFields = isMarket
    ? qty.trim() === ""
    : price.trim() === "" || qty.trim() === "";

  const submitOrder = () => {
    if (hasEmptyFields) return;

    const parsedQty = Number(qty);
    if (!Number.isFinite(parsedQty) || parsedQty < 0) return;

    if (isMarket) {
      onPlaceOrder({
        type: "market",
        qty: Math.trunc(parsedQty),
        side,
      });
      return;
    }

    const parsedPrice = Number(price);
    if (!Number.isFinite(parsedPrice) || parsedPrice < 0) return;

    onPlaceOrder({
      type: "limit",
      price: parsedPrice,
      qty: Math.trunc(parsedQty),
      side,
    });
  };

  const priceSliderValue = price === "" ? 0 : Number(price);
  const qtySliderValue = qty === "" ? 0 : Number(qty);

  return (
    <aside className="panel">
      <h2>Order Entry</h2>

      <div className="order-type-pills">
        <button
          type="button"
          className={`order-type-pill ${side === "buy" ? "active buy" : ""}`}
          onClick={() => setSide("buy")}
        >
          Buy
        </button>
        <button
          type="button"
          className={`order-type-pill ${side === "sell" ? "active sell" : ""}`}
          onClick={() => setSide("sell")}
        >
          Sell
        </button>
      </div>

      <div className="order-type-pills">
        <button
          type="button"
          className={`order-type-pill ${orderType === "limit" ? "active" : ""}`}
          onClick={() => setOrderType("limit")}
        >
          Limit
        </button>
        <button
          type="button"
          className={`order-type-pill ${orderType === "market" ? "active" : ""}`}
          onClick={() => setOrderType("market")}
        >
          Market
        </button>
        <button type="button" className="order-type-pill disabled" disabled>Stop</button>
      </div>

      <div className="order-type-pills">
        <button type="button" className="order-type-pill active">GTC</button>
        <button type="button" className="order-type-pill disabled" disabled>IOC</button>
        <button type="button" className="order-type-pill disabled" disabled>FOK</button>
        <button type="button" className="order-type-pill disabled" disabled>Post</button>
      </div>

      <form className="order-form">
        <label style={{ opacity: isMarket ? 0.4 : 1 }}>
          Price {isMarket && <span style={{ fontSize: "0.6rem", color: "var(--text-dim)" }}>· n/a for market</span>}
          <div className="input-with-slider">
            <input
              type="number"
              min="0"
              step="0.01"
              value={price}
              onChange={(event) => setPrice(event.target.value)}
              disabled={isMarket}
            />
            <input
              type="range"
              min="0"
              max={PRICE_MAX}
              step="0.01"
              value={priceSliderValue}
              onChange={(event) => setPrice(event.target.value)}
              disabled={isMarket}
            />
          </div>
        </label>
        <label>
          Quantity
          <div className="input-with-slider">
            <input
              type="number"
              min="0"
              step="1"
              value={qty}
              onChange={(event) => setQty(event.target.value)}
            />
            <input
              type="range"
              min="0"
              max={QTY_MAX}
              step="1"
              value={qtySliderValue}
              onChange={(event) => setQty(event.target.value)}
            />
          </div>
        </label>
        <div className="actions single-action">
          <button
            type="button"
            className={`place-order ${hasEmptyFields ? "unclickable" : ""}`}
            aria-disabled={hasEmptyFields}
            onClick={submitOrder}
          >
            Place Order
          </button>
        </div>
        <div className="actions">
          <button type="button" onClick={onLogEngine}>
            Log Engine
          </button>
          <button type="button" onClick={onAddTestTrade}>
            Add Test Trade
          </button>
        </div>
      </form>
    </aside>
  );
};

export default OrderEntryPanel;
