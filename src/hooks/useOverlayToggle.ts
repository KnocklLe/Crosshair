import { useState } from "react";
import { api } from "../services/api";
import { store, useStore } from "../services/store";
import { toast } from "../services/toast";

/**
 * Overlay ON/OFF. The displayed state is the one confirmed by the overlay
 * thread (returned by the command), never an optimistic guess.
 */
export function useOverlayToggle() {
  const overlay = useStore((s) => s.overlay);
  const [busy, setBusy] = useState(false);

  const setVisible = async (visible: boolean) => {
    setBusy(true);
    try {
      const status = await api.setOverlayVisible(visible);
      store.set({ overlay: status });
    } catch (e) {
      toast.error((e as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return {
    overlay,
    busy,
    requested: overlay?.requested ?? false,
    visible: overlay?.visible ?? false,
    toggle: () => setVisible(!(overlay?.requested ?? false)),
    setVisible,
  };
}

export type OverlayTone = "on" | "warn" | "err" | "off";

/** Summarizes the confirmed overlay status for display. */
export function overlayTone(o: ReturnType<typeof useOverlayToggle>["overlay"]): OverlayTone {
  if (!o || !o.running || o.lastError) return o?.requested ? "err" : "off";
  if (o.visible) return o.exclusiveFullscreen ? "warn" : "on";
  if (o.requested) return "warn";
  return "off";
}
