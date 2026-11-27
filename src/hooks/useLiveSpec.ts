import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../services/api";
import { store, useStore } from "../services/store";
import { toast } from "../services/toast";
import type { CrosshairSpec } from "../types/models";

const PERSIST_DELAY_MS = 350;
const RECENT_SENT = 24;

/**
 * Live editing of the active profile's spec.
 *
 * * The local draft updates instantly (preview).
 * * The overlay is updated at most once per animation frame, with a single
 *   request in flight: during a drag the latest value always wins.
 * * The database is written once the value has been stable for 350 ms.
 * * External changes (shortcut, profile switch, preset) replace the draft,
 *   except echoes of values this hook sent itself.
 */
export function useLiveSpec() {
  const active = useStore((s) => s.activeProfile);
  const [draft, setDraft] = useState<CrosshairSpec | null>(active?.spec ?? null);

  const sent = useRef<string[]>([]);
  const pending = useRef<CrosshairSpec | null>(null);
  const inFlight = useRef(false);
  const frame = useRef<number | null>(null);
  const persistTimer = useRef<number | null>(null);
  const latest = useRef<CrosshairSpec | null>(draft);
  const profileId = useRef<string | undefined>(active?.id);

  // Resync from the backend when the active profile changes elsewhere.
  useEffect(() => {
    if (!active) return;
    const incoming = JSON.stringify(active.spec);
    if (profileId.current === active.id) {
      // Same profile: ignore echoes of what this hook sent.
      if (latest.current && JSON.stringify(latest.current) === incoming) return;
      if (sent.current.includes(incoming)) return;
    } else {
      // Another profile became active: pending work belonged to the old one.
      profileId.current = active.id;
      sent.current = [];
      pending.current = null;
      if (persistTimer.current !== null) {
        window.clearTimeout(persistTimer.current);
        persistTimer.current = null;
      }
    }
    latest.current = active.spec;
    setDraft(active.spec);
  }, [active]);

  const remember = (json: string) => {
    sent.current.push(json);
    if (sent.current.length > RECENT_SENT) sent.current.shift();
  };

  const pump = useCallback(() => {
    frame.current = null;
    if (inFlight.current || !pending.current) return;
    const spec = pending.current;
    pending.current = null;
    inFlight.current = true;
    remember(JSON.stringify(spec));
    api
      .setActiveSpec(spec, false)
      .then((status) => store.set({ overlay: status }))
      .catch((e: Error) => toast.error(e.message))
      .finally(() => {
        inFlight.current = false;
        if (pending.current && frame.current === null) frame.current = requestAnimationFrame(pump);
      });
  }, []);

  const persist = useCallback(() => {
    persistTimer.current = null;
    const spec = latest.current;
    if (!spec) return;
    remember(JSON.stringify(spec));
    api.setActiveSpec(spec, true).catch((e: Error) => toast.error(e.message));
  }, []);

  const update = useCallback(
    (next: CrosshairSpec) => {
      latest.current = next;
      setDraft(next);
      pending.current = next;
      if (frame.current === null) frame.current = requestAnimationFrame(pump);
      if (persistTimer.current !== null) window.clearTimeout(persistTimer.current);
      persistTimer.current = window.setTimeout(persist, PERSIST_DELAY_MS);
    },
    [pump, persist],
  );

  // Flush a pending save when leaving the page.
  useEffect(
    () => () => {
      if (frame.current !== null) cancelAnimationFrame(frame.current);
      if (persistTimer.current !== null) {
        window.clearTimeout(persistTimer.current);
        persist();
      }
    },
    [persist],
  );

  return { draft, update, profile: active };
}
