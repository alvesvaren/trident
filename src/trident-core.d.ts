// Type augmentation for trident-core WASM module
// This file supplements the wasm-pack generated types which are missing the init exports

declare module "trident-core" {
    export * from "trident-core/trident_core";

    export interface InitInput {
        module_or_path?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module;
    }

    export type InitOutput = typeof import("trident-core/trident_core");

    /**
     * Initialize the WASM module. Must be called before using any exported functions.
     * @param module_or_path - Optional URL or path to the .wasm file, or the WASM bytes/module directly
     */
    export default function init(module_or_path?: InitInput | RequestInfo | URL): Promise<InitOutput>;

    /**
     * Synchronously initialize the WASM module from bytes or a compiled module.
     */
    export function initSync(module: BufferSource | WebAssembly.Module): InitOutput;
}

declare module "trident-core/trident_core_bg.wasm?url" {
    const url: string;
    export default url;
}
