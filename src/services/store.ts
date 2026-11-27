// Minimal global store built on useSyncExternalStore: components subscribe to
// a *slice* of the state, so a change only re-renders what depends on it.

import { useSyncExternalStore } from "react";
import type {
  AppInfo,
  DisplayInfo,
  Limits,
  OverlayStatus,
  PageId,
  Profile,
  Settings,
  ShortcutStatus,
} from "../types/models";

export interface EditorOrigin {
  presetId: string;
  name: string;
  builtin: boolean;
}

export interface AppState {
  ready: boolean;
  fatalError: string | null;
  app: AppInfo | null;
  settings: Settings | null;
  limits: Limits | null;
  overlay: OverlayStatus | null;
  activeProfile: Profile | null;
  profileCount: number;
  shortcuts: ShortcutStatus[];
  displays: DisplayInfo[];
  page: PageId;
  /** Set when the editor was opened from a library preset. */
  editorOrigin: EditorOrigin | null;
  /** Incremented after any backend state change: lists refetch on it. */
  dataVersion: number;
}

type Listener = () => void;

const initial: AppState = {
  ready: false,
  fatalError: null,
  app: null,
  settings: null,
  limits: null,
  overlay: null,
  activeProfile: null,
  profileCount: 0,
  shortcuts: [],
  displays: [],
  page: "dashboard",
  editorOrigin: null,
  dataVersion: 0,
};

let state: AppState = initial;
const listeners = new Set<Listener>();

export const store = {
  get: (): AppState => state,
  set(patch: Partial<AppState> | ((s: AppState) => Partial<AppState>)) {
    const next = typeof patch === "function" ? patch(state) : patch;
    let changed = false;
    for (const key of Object.keys(next) as (keyof AppState)[]) {
      if (!Object.is(state[key], next[key])) {
        changed = true;
        break;
      }
    }
    if (!changed) return;
    state = { ...state, ...next };
    listeners.forEach((l) => l());
  },
  subscribe(listener: Listener) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  bumpData() {
    store.set((s) => ({ dataVersion: s.dataVersion + 1 }));
  },
  navigate(page: PageId, editorOrigin: EditorOrigin | null = null) {
    store.set({ page, editorOrigin });
  },
};

/** Subscribes to a slice of the state. Selectors must return stable values. */
export function useStore<T>(selector: (s: AppState) => T): T {
  return useSyncExternalStore(store.subscribe, () => selector(state), () => selector(initial));
}
