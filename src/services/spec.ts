// Helpers around CrosshairSpec. Defaults mirror src-tauri/src/crosshair/model.rs.

import type { CrosshairConfig, CrosshairShape, CrosshairSpec, DotConfig } from "../types/models";

export const DEFAULT_DOT: DotConfig = {
  size: 4,
  color: "#00FF66",
  opacity: 1,
  shape: "circle",
  outline: { enabled: true, color: "#000000", thickness: 1 },
  halo: { enabled: false, color: "#FFFFFF", size: 6, opacity: 0.35 },
};

export const DEFAULT_CROSSHAIR: CrosshairConfig = {
  shape: "cross_gap",
  lengthH: 6,
  lengthV: 6,
  thickness: 2,
  gap: 3,
  color: "#00FF66",
  opacity: 1,
  rotation: 0,
  outline: { enabled: true, color: "#000000", thickness: 1 },
  shadow: { enabled: false, color: "#000000", opacity: 0.6, offsetX: 1, offsetY: 1, blur: 2 },
  dotSize: 2,
  dotShape: "square",
  circleRadius: 10,
  circleThickness: 1.5,
  splitGap: 3,
  custom: { top: true, right: true, bottom: true, left: true, circle: false, dot: false },
};

export function defaultSpec(): CrosshairSpec {
  return structuredClone({ kind: "crosshair", dot: DEFAULT_DOT, crosshair: DEFAULT_CROSSHAIR });
}

export const SHAPES: CrosshairShape[] = [
  "cross",
  "cross_gap",
  "cross_dynamic",
  "cross_dot",
  "circle",
  "circle_dot",
  "dot",
  "custom",
];

/** Which parameter groups are meaningful for a shape (the editor hides the others). */
export function shapeFeatures(c: CrosshairConfig) {
  const s = c.shape;
  const custom = s === "custom";
  const arms = s === "cross" || s === "cross_gap" || s === "cross_dynamic" || s === "cross_dot" || custom;
  return {
    arms,
    gap: arms && s !== "cross",
    circle: s === "circle" || s === "circle_dot" || (custom && c.custom.circle),
    dot: s === "dot" || s === "circle_dot" || s === "cross_dot" || (custom && c.custom.dot),
    dynamic: s === "cross_dynamic",
    custom,
  };
}

/** Immutable deep update: `patchCrosshair(spec, { outline: { enabled: true } })`. */
export function patchCrosshair(spec: CrosshairSpec, patch: DeepPartial<CrosshairConfig>): CrosshairSpec {
  return { ...spec, kind: "crosshair", crosshair: merge(spec.crosshair, patch) };
}

export function patchDot(spec: CrosshairSpec, patch: DeepPartial<DotConfig>): CrosshairSpec {
  return { ...spec, kind: "dot", dot: merge(spec.dot, patch) };
}

export type DeepPartial<T> = { [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K] };

function merge<T extends object>(base: T, patch: DeepPartial<T>): T {
  const out = { ...base } as Record<string, unknown>;
  for (const [k, v] of Object.entries(patch)) {
    const current = out[k];
    out[k] =
      v && typeof v === "object" && !Array.isArray(v) && current && typeof current === "object"
        ? merge(current as object, v as object)
        : v;
  }
  return out as T;
}

export function isHexColor(s: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(s);
}

/** Accepts "abc", "#abc", "aabbcc"… and returns "#AABBCC", or null. */
export function normalizeHex(input: string): string | null {
  let s = input.trim().replace(/^#/, "");
  if (/^[0-9a-fA-F]{3}$/.test(s)) s = s.split("").map((c) => c + c).join("");
  return /^[0-9a-fA-F]{6}$/.test(s) ? `#${s.toUpperCase()}` : null;
}

export function clamp(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}

/** Rounds to the slider step, avoiding floating-point noise (0.30000000004). */
export function snapToStep(v: number, step: number, min = 0): number {
  if (step <= 0) return v;
  const decimals = (String(step).split(".")[1] ?? "").length;
  return Number((Math.round((v - min) / step) * step + min).toFixed(decimals));
}
