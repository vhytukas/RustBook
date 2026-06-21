/* tslint:disable */
/* eslint-disable */

export class WasmEngine {
    free(): void;
    [Symbol.dispose](): void;
    amend_order_qty(order_id: bigint, new_qty: bigint): void;
    burst(n: bigint): any;
    cancel_order(order_id: bigint): bigint;
    clear_risk_gate(): void;
    static default_risk_gate(): any;
    drain_trades(): any;
    constructor();
    orderbook_depth_state(): any;
    orderbook_full_state(): any;
    place_limit_order(price: bigint, qty: bigint, side: WasmSide): bigint;
    place_market_order(qty: bigint, side: WasmSide): BigUint64Array;
    static price_scale(): bigint;
    set_risk_gate(config_js: any): void;
    simulation_active(): boolean;
    start_simulation(config_js: any): void;
    trades(): any;
}

export class WasmReplayer {
    free(): void;
    [Symbol.dispose](): void;
    cursor(): bigint;
    drain_trades(): any;
    constructor(config_js: any);
    orderbook_depth_state(): any;
    reset(): void;
    seek(target: bigint): void;
    /**
     * Apply the next event. Returns dt_nanos so JS can pace playback.
     */
    step(): bigint;
}

export enum WasmSide {
    Buy = 0,
    Sell = 1,
}

export function wasm_start(): void;
