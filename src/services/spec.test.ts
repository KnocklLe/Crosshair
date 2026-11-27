import { describe, expect, it } from "vitest";
import fixture from "./default-spec.json";
import { defaultSpec, normalizeHex, patchCrosshair, patchDot, shapeFeatures, snapToStep } from "./spec";

describe("spec helpers", () => {
  it("defaults are independent copies", () => {
    const a = defaultSpec();
    a.crosshair.outline.color = "#FF0000";
    expect(defaultSpec().crosshair.outline.color).toBe("#000000");
  });

  it("patchCrosshair deep-merges and switches to crosshair mode", () => {
    const base = { ...defaultSpec(), kind: "dot" as const };
    const next = patchCrosshair(base, { outline: { thickness: 2 } });
    expect(next.kind).toBe("crosshair");
    expect(next.crosshair.outline).toEqual({ enabled: true, color: "#000000", thickness: 2 });
    expect(base.crosshair.outline.thickness).toBe(1); // immutable
    expect(next.dot).toBe(base.dot); // untouched branch is shared
  });

  it("patchDot switches to dot mode and keeps the crosshair", () => {
    const base = defaultSpec();
    const next = patchDot(base, { size: 9, halo: { enabled: true } });
    expect(next.kind).toBe("dot");
    expect(next.dot.size).toBe(9);
    expect(next.dot.halo.enabled).toBe(true);
    expect(next.dot.halo.size).toBe(6);
    expect(next.crosshair).toBe(base.crosshair);
  });

  it("shape features drive which controls are shown", () => {
    const c = defaultSpec().crosshair;
    expect(shapeFeatures({ ...c, shape: "cross" })).toMatchObject({ arms: true, gap: false, dot: false });
    expect(shapeFeatures({ ...c, shape: "circle_dot" })).toMatchObject({ arms: false, circle: true, dot: true });
    expect(shapeFeatures({ ...c, shape: "cross_dynamic" })).toMatchObject({ dynamic: true, gap: true });
    expect(shapeFeatures({ ...c, shape: "custom", custom: { ...c.custom, circle: true } })).toMatchObject({ circle: true, dot: false, custom: true });
  });

  it("normalizes colours", () => {
    expect(normalizeHex("#00ff66")).toBe("#00FF66");
    expect(normalizeHex("fa0")).toBe("#FFAA00");
    expect(normalizeHex("#12345")).toBeNull();
    expect(normalizeHex("green")).toBeNull();
  });

  it("snaps values to the slider step", () => {
    expect(snapToStep(0.30000000004, 0.01)).toBe(0.3);
    expect(snapToStep(2.26, 0.5, 0.5)).toBe(2.5);
    expect(snapToStep(7.4, 1)).toBe(7);
  });
});

describe("defaults shared with Rust", () => {
  it("match the fixture also checked by the Rust tests", () => {
    expect(defaultSpec()).toEqual(fixture);
  });
});
