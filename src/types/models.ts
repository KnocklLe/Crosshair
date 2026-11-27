// TypeScript mirror of the Rust models (src-tauri/src). Field names follow
// the serde `camelCase` / `snake_case` attributes exactly.

export type SpecKind = "dot" | "crosshair";
export type DotShape = "circle" | "square";
export type CrosshairShape =
  | "cross"
  | "cross_gap"
  | "cross_dynamic"
  | "circle"
  | "circle_dot"
  | "dot"
  | "cross_dot"
  | "custom";

export interface Outline {
  enabled: boolean;
  color: string;
  thickness: number;
}

export interface Halo {
  enabled: boolean;
  color: string;
  size: number;
  opacity: number;
}

export interface Shadow {
  enabled: boolean;
  color: string;
  opacity: number;
  offsetX: number;
  offsetY: number;
  blur: number;
}

export interface DotConfig {
  size: number;
  color: string;
  opacity: number;
  shape: DotShape;
  outline: Outline;
  halo: Halo;
}

export interface CustomElements {
  top: boolean;
  right: boolean;
  bottom: boolean;
  left: boolean;
  circle: boolean;
  dot: boolean;
}

export interface CrosshairConfig {
  shape: CrosshairShape;
  lengthH: number;
  lengthV: number;
  thickness: number;
  gap: number;
  color: string;
  opacity: number;
  rotation: number;
  outline: Outline;
  shadow: Shadow;
  dotSize: number;
  dotShape: DotShape;
  circleRadius: number;
  circleThickness: number;
  splitGap: number;
  custom: CustomElements;
}

export interface CrosshairSpec {
  kind: SpecKind;
  dot: DotConfig;
  crosshair: CrosshairConfig;
}

export interface Profile {
  id: string;
  name: string;
  description: string;
  spec: CrosshairSpec;
  sortIndex: number;
  createdAt: number;
  updatedAt: number;
}

export interface ProfileInput {
  name: string;
  description: string;
  spec?: CrosshairSpec;
}

export type PresetCategory = "minimal" | "competitive" | "classic" | "dot" | "circle" | "custom";

export const PRESET_CATEGORIES: PresetCategory[] = ["minimal", "competitive", "classic", "dot", "circle", "custom"];

export interface Preset {
  id: string;
  name: string;
  category: PresetCategory;
  spec: CrosshairSpec;
  favorite: boolean;
  builtin: boolean;
  createdAt: number;
  lastUsedAt: number | null;
}

export interface PresetInput {
  name: string;
  category: PresetCategory;
  spec: CrosshairSpec;
}

export interface RecentEntry {
  id: number;
  name: string;
  source: "preset" | "profile" | string;
  sourceId: string | null;
  spec: CrosshairSpec;
  usedAt: number;
}

export interface OverlayStatus {
  running: boolean;
  visible: boolean;
  requested: boolean;
  autoHidden: boolean;
  monitorId: string | null;
  monitorName: string | null;
  monitorFallback: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  renderCount: number;
  lastRenderMicros: number;
  exclusiveFullscreen: boolean;
  lastError: string | null;
}

export interface DisplayInfo {
  id: string;
  name: string;
  index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  dpi: number;
  scale: number;
  refreshRate: number;
  primary: boolean;
}

export type Language = "auto" | "en" | "fr";
export type Theme = "dark" | "light" | "system";
export type VisibilityMode = "always" | "fullscreen_only";

export interface OverlaySettings {
  autoActivate: boolean;
  monitor: string | null;
  visibility: VisibilityMode;
  keepOnTop: boolean;
  topmostIntervalMs: number;
  dpiScaling: boolean;
  offsetX: number;
  offsetY: number;
}

export interface DotSizePreset {
  id: string;
  name: string;
  size: number;
}

export interface Settings {
  version: number;
  language: Language;
  theme: Theme;
  launchAtStartup: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  overlay: OverlaySettings;
  performance: { liteMode: boolean };
  dotPresets: DotSizePreset[];
}

export type ShortcutAction = "toggle_overlay" | "toggle_dot" | "next_profile";

export interface ShortcutStatus {
  action: ShortcutAction;
  accelerator: string;
  enabled: boolean;
  registered: boolean;
  error: string | null;
}

export interface Range {
  min: number;
  max: number;
  step: number;
}

export interface Limits {
  dotSize: Range;
  opacity: Range;
  outlineThickness: Range;
  haloSize: Range;
  length: Range;
  thickness: Range;
  gap: Range;
  rotation: Range;
  shadowOffset: Range;
  shadowBlur: Range;
  centerDotSize: Range;
  circleRadius: Range;
  circleThickness: Range;
  splitGap: Range;
  nameMaxChars: number;
  descriptionMaxChars: number;
}

export interface AppInfo {
  name: string;
  version: string;
  tauriVersion: string;
  dataDir: string;
  dbPath: string;
  schemaVersion: number;
  launchedAtStartup: boolean;
}

export interface StateSnapshot {
  overlay: OverlayStatus;
  activeProfile: Profile;
  profileCount: number;
  source: "ui" | "shortcut" | "tray" | "startup" | string;
}

export interface Bootstrap {
  app: AppInfo;
  settings: Settings;
  limits: Limits;
  state: StateSnapshot;
  shortcuts: ShortcutStatus[];
  displays: DisplayInfo[];
}

export interface Dashboard {
  state: StateSnapshot;
  presetCount: number;
  customPresetCount: number;
  favoriteCount: number;
  recents: RecentEntry[];
  shortcuts: ShortcutStatus[];
  displayCount: number;
}

export interface ProcessUsage {
  workingSetBytes: number;
  privateBytes: number;
  cpuPercent: number;
  uptimeSeconds: number;
  logicalCpus: number;
}

export interface PerformanceInfo {
  process: ProcessUsage;
  overlay: OverlayStatus;
}

export interface SkippedItem {
  kind: string;
  name: string;
  reason: string;
}

export interface ImportReport {
  profilesImported: number;
  presetsImported: number;
  settingsRestored: boolean;
  skipped: SkippedItem[];
}

export interface Warning {
  code: string;
  message: string;
}

/** Error payload returned by every failing command. */
export interface AppErrorPayload {
  code: string;
  message: string;
}

export type PageId = "dashboard" | "editor" | "dot" | "library" | "profiles" | "settings" | "about";
