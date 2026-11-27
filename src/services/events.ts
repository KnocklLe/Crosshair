// Subscribes to the events emitted by the Rust backend and keeps the store in
// sync. State shown in the UI always comes from these confirmed snapshots.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DisplayInfo, OverlayStatus, ShortcutStatus, StateSnapshot, Warning } from "../types/models";
import { store } from "./store";
import { toast } from "./toast";
import { translate } from "../i18n";

export async function subscribeBackendEvents(): Promise<UnlistenFn> {
  const unlisteners = await Promise.all([
    listen<StateSnapshot>("app://state", ({ payload }) => {
      const prev = store.get();
      store.set({
        overlay: payload.overlay,
        activeProfile: payload.activeProfile,
        profileCount: payload.profileCount,
      });
      store.bumpData();
      if (payload.source === "shortcut" || payload.source === "tray") {
        announceExternalChange(prev.activeProfile?.id, prev.overlay?.requested, payload);
      }
    }),
    listen<OverlayStatus>("app://overlay", ({ payload }) => store.set({ overlay: payload })),
    listen<DisplayInfo[]>("app://displays", ({ payload }) => {
      store.set({ displays: payload });
      toast.info(translate("toast.displaysChanged"));
    }),
    listen<ShortcutStatus[]>("app://shortcuts", ({ payload }) => store.set({ shortcuts: payload })),
    listen<Warning>("app://warning", ({ payload }) => {
      const key = `warning.${payload.code}`;
      const localized = translate(key);
      const message =
        localized === key ? payload.message : payload.code === "action_failed" ? `${localized} ${payload.message}` : localized;
      toast.warning(message, payload.code === "exclusive_fullscreen" ? 9000 : 6000);
    }),
  ]);
  return () => unlisteners.forEach((u) => u());
}

function announceExternalChange(prevProfileId: string | undefined, prevRequested: boolean | undefined, s: StateSnapshot) {
  if (prevProfileId && prevProfileId !== s.activeProfile.id) {
    toast.info(translate("toast.profileActivated", { name: s.activeProfile.name }));
  } else if (prevRequested !== undefined && prevRequested !== s.overlay.requested) {
    toast.info(translate(s.overlay.requested ? "toast.overlayOn" : "toast.overlayOff"));
  } else {
    toast.info(translate(s.activeProfile.spec.kind === "dot" ? "toast.dotModeOn" : "toast.crosshairModeOn"));
  }
}
