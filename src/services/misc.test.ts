import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { decodeRender } from "./api";
import { formatBytes, formatDuration, formatMicros } from "./format";
import { store } from "./store";

describe("binary preview decoding", () => {
  it("decodes the [w][h][rgba] format", () => {
    const buf = new ArrayBuffer(8 + 2 * 2 * 4);
    const view = new DataView(buf);
    view.setUint32(0, 2, true);
    view.setUint32(4, 2, true);
    new Uint8Array(buf, 8).fill(255);
    const img = decodeRender(buf);
    expect(img.width).toBe(2);
    expect(img.height).toBe(2);
    expect(img.pixels.length).toBe(16);
  });

  it("rejects truncated data", () => {
    const buf = new ArrayBuffer(8 + 4);
    const view = new DataView(buf);
    view.setUint32(0, 2, true);
    view.setUint32(4, 2, true);
    expect(() => decodeRender(buf)).toThrow(/Invalid render size/);
    expect(() => decodeRender(new ArrayBuffer(3))).toThrow();
  });
});

describe("formatting", () => {
  it("formats bytes, durations and render times", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(42 * 1024 * 1024)).toBe("42.0 MB");
    expect(formatDuration(59)).toBe("59 s");
    expect(formatDuration(125)).toBe("2 min 05 s");
    expect(formatDuration(3720)).toBe("1 h 02 min");
    expect(formatMicros(250)).toBe("250 µs");
    expect(formatMicros(1830)).toBe("1.83 ms");
  });
});

describe("store", () => {
  it("notifies only on real changes", () => {
    const listener = vi.fn();
    const unsubscribe = store.subscribe(listener);
    store.set({ profileCount: 3 });
    store.set({ profileCount: 3 });
    expect(listener).toHaveBeenCalledTimes(1);
    store.navigate("library");
    expect(store.get().page).toBe("library");
    expect(listener).toHaveBeenCalledTimes(2);
    unsubscribe();
  });
});
