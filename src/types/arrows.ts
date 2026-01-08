/**
 * Arrow type definitions - mirrors the Rust ArrowEntry structure
 */

/** Line style for arrow rendering */
export type LineStyle = "solid" | "dashed";

/** Head/marker style for arrow endpoints */
export type HeadStyle = "none" | "arrow" | "triangle" | "diamond_filled" | "diamond_empty";

/** Direction of an arrow */
export type ArrowDirection = "right" | "left" | "none";

/** Complete arrow entry from the Rust registry */
export interface ArrowEntry {
  /** Token string for parsing (e.g., "-->", "<--") */
  token: string;
  /** Canonical name with direction suffix (e.g., "assoc_right", "assoc_left") */
  canonical_name: string;
  /** Base name without direction suffix */
  name: string;
  /** Human-readable label for autocomplete dropdown */
  label: string;
  /** Detailed description for autocomplete */
  detail: string;
  /** Line style (solid or dashed) */
  line_style: LineStyle;
  /** Head style at the "to" end */
  head_style: HeadStyle;
  /** Head style at the "from" end (for diamonds) */
  tail_style: HeadStyle;
  /** Direction of the arrow */
  direction: ArrowDirection;
  /** Whether this is a left arrow variant */
  is_left: boolean;
}

/** Cached arrow registry from Rust */
let arrowRegistry: ArrowEntry[] | null = null;

/** Initialize the arrow registry from Rust WASM module */
export function initArrowRegistry(tridentCore: { get_arrows: () => string }): void {
  const json = tridentCore.get_arrows();
  arrowRegistry = JSON.parse(json) as ArrowEntry[];
}

/** Get the arrow registry (must call initArrowRegistry first) */
export function getArrowRegistry(): ArrowEntry[] {
  if (!arrowRegistry) {
    throw new Error("Arrow registry not initialized. Call initArrowRegistry first.");
  }
  return arrowRegistry;
}

/** Get arrow entry by canonical name */
export function getArrowByName(canonicalName: string): ArrowEntry | undefined {
  return getArrowRegistry().find(e => e.canonical_name === canonicalName);
}

/** Get arrow entry by token */
export function getArrowByToken(token: string): ArrowEntry | undefined {
  return getArrowRegistry().find(e => e.token === token);
}

/** Get all arrow tokens (sorted by length, longest first - for parsing) */
export function getArrowTokens(): string[] {
  return getArrowRegistry().map(e => e.token);
}

/** Check if an arrow should be rendered with dashed line */
export function isDashed(canonicalName: string): boolean {
  const arrow = getArrowByName(canonicalName);
  return arrow?.line_style === "dashed";
}

/** Check if an arrow is a "left" arrow */
export function isLeftArrow(canonicalName: string): boolean {
  const arrow = getArrowByName(canonicalName);
  return arrow?.is_left ?? false;
}

/** Get the base arrow name (without _left/_right suffix) */
export function getBaseArrowName(canonicalName: string): string {
  const arrow = getArrowByName(canonicalName);
  return arrow?.name ?? canonicalName;
}
