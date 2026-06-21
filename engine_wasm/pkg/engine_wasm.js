/* @ts-self-types="./engine_wasm.d.ts" */

import * as wasm from "./engine_wasm_bg.wasm";
import { __wbg_set_wasm } from "./engine_wasm_bg.js";
__wbg_set_wasm(wasm);
wasm.__wbindgen_start();
export {
    WasmEngine, WasmReplayer, WasmSide, wasm_start
} from "./engine_wasm_bg.js";
