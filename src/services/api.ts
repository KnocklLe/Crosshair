// Typed wrappers around the Tauri commands. This is the only file that calls
// `invoke`: pages and components never talk to the backend directly.

import { invoke } from "@tauri-apps/api/core";
import type {
  AppErrorPayload,
  Bootstrap,
  CrosshairSpec,
  Dashboard,
  DisplayInfo,
  ImportReport,
  OverlayStatus,
  PerformanceInfo,
  Preset,
  PresetInput,
  Profile,
  ProfileInput,
  RecentEntry,
  Settings,
  ShortcutAction,
  ShortcutStatus,
} from "../types/models";

export class ApiError extends Error {
  code: string;
  constructor(payload: AppErrorPayload) {
    super(payload.message);
    this.code = payload.code;
  }
}

function toApiError(e: unknown): ApiError {
  if (e instanceof ApiError) return e;
  if (e && typeof e === "object" && "message" in e) {
    const p = e as Partial<AppErrorPayload>;
    return new ApiError({ code: p.code ?? "unknown", message: String(p.message) });
  }
  return new ApiError({ code: "unknown", message: String(e) });
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toApiError(e);
  }
}

export interface RenderedImage {
  width: number;
  height: number;
  /** Straight-alpha RGBA pixels. */
  pixels: Uint8ClampedArray<ArrayBuffer>;
}

/** Decodes the binary preview format: [w u32 LE][h u32 LE][RGBA…]. */
export function decodeRender(buffer: ArrayBuffer): RenderedImage {
  if (buffer.byteLength < 8) throw new ApiError({ code: "render", message: "Empty render" });
  const view = new DataView(buffer);
  const width = view.getUint32(0, true);
  const height = view.getUint32(4, true);
  const expected = 8 + width * height * 4;
  if (buffer.byteLength !== expected) {
    throw new ApiError({ code: "render", message: `Invalid render size (${buffer.byteLength} != ${expected})` });
  }
  return { width, height, pixels: new Uint8ClampedArray(buffer, 8) };
}

export const api = {
  // app
  getBootstrap: () => call<Bootstrap>("get_bootstrap"),
  appReady: (language: string) => call<void>("app_ready", { language }),
  setUiLanguage: (language: string) => call<void>("set_ui_language", { language }),
  getDashboard: () => call<Dashboard>("get_dashboard"),
  getPerformance: () => call<PerformanceInfo>("get_performance"),
  openDataFolder: () => call<void>("open_data_folder"),
  quitApp: () => call<void>("quit_app"),
  listRecents: (limit?: number) => call<RecentEntry[]>("list_recents", { limit }),
  applyRecent: (id: number) => call<Profile>("apply_recent", { id }),

  // overlay
  getOverlayStatus: () => call<OverlayStatus>("get_overlay_status"),
  setOverlayVisible: (visible: boolean) => call<OverlayStatus>("set_overlay_visible", { visible }),
  toggleOverlay: () => call<OverlayStatus>("toggle_overlay"),
  refreshOverlay: () => call<OverlayStatus>("refresh_overlay"),

  // crosshair
  renderPreview: async (spec: CrosshairSpec, scale = 1) =>
    decodeRender(await call<ArrayBuffer>("render_preview", { spec, scale })),
  setActiveSpec: (spec: CrosshairSpec, persist: boolean) =>
    call<OverlayStatus>("set_active_spec", { spec, persist }),
  toggleDotMode: () => call<Profile>("toggle_dot_mode"),

  // profiles
  listProfiles: () => call<Profile[]>("list_profiles"),
  createProfile: (input: ProfileInput) => call<Profile>("create_profile", { input }),
  updateProfile: (id: string, input: ProfileInput) => call<Profile>("update_profile", { id, input }),
  duplicateProfile: (id: string) => call<Profile>("duplicate_profile", { id }),
  deleteProfile: (id: string) => call<void>("delete_profile", { id }),
  activateProfile: (id: string) => call<Profile>("activate_profile", { id }),
  nextProfile: () => call<Profile>("next_profile"),
  exportProfiles: (title: string, ids?: string[]) => call<string | null>("export_profiles", { ids, title }),
  importProfiles: (title: string) => call<ImportReport | null>("import_profiles", { title }),

  // library
  listPresets: () => call<Preset[]>("list_presets"),
  applyPreset: (id: string) => call<Profile>("apply_preset", { id }),
  createPreset: (input: PresetInput) => call<Preset>("create_preset", { input }),
  updatePreset: (id: string, input: PresetInput) => call<Preset>("update_preset", { id, input }),
  duplicatePreset: (id: string) => call<Preset>("duplicate_preset", { id }),
  deletePreset: (id: string) => call<void>("delete_preset", { id }),
  setPresetFavorite: (id: string, favorite: boolean) => call<Preset>("set_preset_favorite", { id, favorite }),
  restoreBuiltinPresets: () => call<number>("restore_builtin_presets"),

  // settings & data
  getSettings: () => call<Settings>("get_settings"),
  updateSettings: (settings: Settings) => call<Settings>("update_settings", { settings }),
  resetAllData: () => call<Settings>("reset_all_data"),
  exportBackup: (title: string) => call<string | null>("export_backup", { title }),
  importBackup: (title: string, restoreSettings: boolean) =>
    call<ImportReport | null>("import_backup", { title, restoreSettings }),

  // displays & shortcuts
  listDisplays: () => call<DisplayInfo[]>("list_displays"),
  listShortcuts: () => call<ShortcutStatus[]>("list_shortcuts"),
  setShortcut: (action: ShortcutAction, accelerator: string, enabled: boolean) =>
    call<ShortcutStatus[]>("set_shortcut", { action, accelerator, enabled }),
  resetShortcuts: () => call<ShortcutStatus[]>("reset_shortcuts"),
};
