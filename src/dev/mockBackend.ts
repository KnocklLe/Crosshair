// DEVELOPMENT ONLY — lets the interface run in a plain browser (outside
// Tauri) for visual checks. Loaded by main.tsx only when `import.meta.env.DEV`
// is true and no Tauri runtime is present; it is never part of a production
// build. The crosshair raster below is a rough approximation: the real
// renderer is the Rust engine.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { defaultSpec } from "../services/spec";
import type { CrosshairSpec, Preset, Profile } from "../types/models";

const now = Date.now();
let profiles: Profile[] = [
  { id: "p1", name: "Default", description: "", spec: defaultSpec(), sortIndex: 0, createdAt: now - 864e5, updatedAt: now },
  {
    id: "p2",
    name: "Valorant",
    description: "Cyan, thin, competitive",
    spec: { ...defaultSpec(), crosshair: { ...defaultSpec().crosshair, color: "#00FFFF", thickness: 1 } },
    sortIndex: 1,
    createdAt: now - 4e7,
    updatedAt: now - 1e6,
  },
];
let activeId = "p1";
const presets: Preset[] = ["minimal", "competitive", "classic", "dot", "circle", "custom"].flatMap((category, i) =>
  [0, 1].map((j) => {
    const spec = defaultSpec();
    if (category === "dot") spec.kind = "dot";
    if (category === "circle") spec.crosshair.shape = "circle_dot";
    spec.crosshair.color = ["#00FF66", "#00FFFF", "#FFFF00", "#FF00FF"][(i + j) % 4];
    return {
      id: `${category}-${j}`,
      name: `${category[0].toUpperCase()}${category.slice(1)} ${j + 1}`,
      category: category as Preset["category"],
      spec,
      favorite: j === 0 && i % 2 === 0,
      builtin: true,
      createdAt: now,
      lastUsedAt: null,
    };
  }),
);

const overlay = {
  running: true,
  visible: true,
  requested: true,
  autoHidden: false,
  monitorId: "\\\\.\\DISPLAY1",
  monitorName: "DELL U2723QE",
  monitorFallback: false,
  x: 1900,
  y: 1060,
  width: 40,
  height: 40,
  scale: 1,
  renderCount: 3,
  lastRenderMicros: 180,
  exclusiveFullscreen: false,
  lastError: null,
};

let settings = {
  version: 1,
  language: "auto",
  theme: "dark",
  launchAtStartup: false,
  startMinimized: true,
  closeToTray: true,
  overlay: { autoActivate: true, monitor: null, visibility: "always", keepOnTop: true, topmostIntervalMs: 2000, dpiScaling: false, offsetX: 0, offsetY: 0 },
  performance: { liteMode: false },
  dotPresets: [
    { id: "micro", name: "Micro", size: 2 },
    { id: "small", name: "Small", size: 4 },
    { id: "medium", name: "Medium", size: 6 },
    { id: "large", name: "Large", size: 9 },
  ],
};

const r = (min: number, max: number, step: number) => ({ min, max, step });
const limits = {
  dotSize: r(1, 64, 1),
  opacity: r(0, 1, 0.01),
  outlineThickness: r(0.5, 8, 0.5),
  haloSize: r(1, 48, 1),
  length: r(0, 200, 1),
  thickness: r(0.5, 24, 0.5),
  gap: r(0, 120, 1),
  rotation: r(-180, 180, 1),
  shadowOffset: r(-20, 20, 1),
  shadowBlur: r(0, 16, 1),
  centerDotSize: r(0, 32, 1),
  circleRadius: r(1, 150, 1),
  circleThickness: r(0.5, 16, 0.5),
  splitGap: r(0, 40, 1),
  nameMaxChars: 64,
  descriptionMaxChars: 280,
};

const shortcuts = [
  { action: "toggle_overlay", accelerator: "Ctrl+Shift+H", enabled: true, registered: true, error: null },
  { action: "toggle_dot", accelerator: "Ctrl+Shift+D", enabled: true, registered: true, error: null },
  { action: "next_profile", accelerator: "Ctrl+Shift+P", enabled: true, registered: false, error: "Ctrl+Shift+P is already used by another application." },
];

const displays = [
  { id: "\\\\.\\DISPLAY1", name: "DELL U2723QE", index: 1, x: 0, y: 0, width: 3840, height: 2160, dpi: 144, scale: 1.5, refreshRate: 60, primary: true },
  { id: "\\\\.\\DISPLAY2", name: "Generic PnP Monitor", index: 2, x: 3840, y: 0, width: 1920, height: 1080, dpi: 96, scale: 1, refreshRate: 144, primary: false },
];

const hex = (c: string) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16));

/** Very rough raster (rectangles only) — enough to see the layout. */
function rasterize(spec: CrosshairSpec): ArrayBuffer {
  const size = 64;
  const px = new Uint8ClampedArray(size * size * 4);
  const fill = (x0: number, y0: number, w: number, h: number, rgb: number[], a: number) => {
    for (let y = Math.max(0, Math.round(y0)); y < Math.min(size, Math.round(y0 + h)); y++)
      for (let x = Math.max(0, Math.round(x0)); x < Math.min(size, Math.round(x0 + w)); x++) {
        const i = (y * size + x) * 4;
        px.set([rgb[0], rgb[1], rgb[2], Math.round(a * 255)], i);
      }
  };
  const c = size / 2;
  const draw = (expand: number, rgb: number[], a: number) => {
    if (spec.kind === "dot") {
      const s = spec.dot.size + expand * 2;
      fill(c - s / 2, c - s / 2, s, s, rgb, a);
      return;
    }
    const k = spec.crosshair;
    const t = k.thickness + expand * 2;
    const inner = k.thickness / 2 + k.gap - expand;
    const len = Math.min(k.lengthH, 24) + expand * 2;
    fill(c - inner - len, c - t / 2, len, t, rgb, a);
    fill(c + inner, c - t / 2, len, t, rgb, a);
    fill(c - t / 2, c - inner - len, t, len, rgb, a);
    fill(c - t / 2, c + inner, t, len, rgb, a);
  };
  const cfg = spec.kind === "dot" ? spec.dot : spec.crosshair;
  if (cfg.outline.enabled) draw(cfg.outline.thickness, hex(cfg.outline.color), cfg.opacity);
  draw(0, hex(cfg.color), cfg.opacity);
  const out = new ArrayBuffer(8 + px.length);
  new DataView(out).setUint32(0, size, true);
  new DataView(out).setUint32(4, size, true);
  new Uint8Array(out, 8).set(px);
  return out;
}

const active = () => profiles.find((p) => p.id === activeId)!;
const snapshot = () => ({ overlay, activeProfile: active(), profileCount: profiles.length, source: "ui" });

export function installMockBackend() {
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "get_bootstrap":
          return {
            app: { name: "Crosshair Studio", version: "1.0.0", tauriVersion: "2.12.7", dataDir: "C:\\Users\\demo\\AppData\\Roaming\\com.crosshairstudio.app", dbPath: "C:\\Users\\demo\\AppData\\Roaming\\com.crosshairstudio.app\\crosshair-studio.db", schemaVersion: 1, launchedAtStartup: false },
            settings,
            limits,
            state: snapshot(),
            shortcuts,
            displays,
          };
        case "get_dashboard":
          return {
            state: snapshot(),
            presetCount: presets.length,
            customPresetCount: 0,
            favoriteCount: presets.filter((p) => p.favorite).length,
            recents: presets.slice(0, 5).map((p, i) => ({ id: i, name: p.name, source: "preset", sourceId: p.id, spec: p.spec, usedAt: now - i * 6e5 })),
            shortcuts,
            displayCount: displays.length,
          };
        case "render_preview":
          return rasterize(a.spec as CrosshairSpec);
        case "set_active_spec":
          active().spec = a.spec as CrosshairSpec;
          return overlay;
        case "set_overlay_visible":
          overlay.requested = overlay.visible = a.visible as boolean;
          return overlay;
        case "list_profiles":
          return profiles;
        case "activate_profile":
          activeId = a.id as string;
          return active();
        case "list_presets":
          return presets;
        case "list_displays":
          return displays;
        case "list_shortcuts":
          return shortcuts;
        case "get_settings":
          return settings;
        case "update_settings":
          settings = a.settings as typeof settings;
          return settings;
        case "get_performance":
          return { process: { workingSetBytes: 41e6, privateBytes: 28e6, cpuPercent: 0.1, uptimeSeconds: 754, logicalCpus: 16 }, overlay };
        case "toggle_dot_mode":
          active().spec = { ...active().spec, kind: active().spec.kind === "dot" ? "crosshair" : "dot" };
          return active();
        case "delete_profile":
          profiles = profiles.filter((p) => p.id !== a.id);
          return null;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
