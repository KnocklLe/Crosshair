import { useEffect, useRef, useState } from "react";
import { displayParts, fromKeyEvent } from "../services/accelerator";
import { useI18n } from "../hooks/useI18n";
import { Kbd } from "./controls";
import { Keyboard } from "./icons";

/** Click, then press a key combination. Esc cancels. */
export function ShortcutRecorder(props: {
  value: string;
  disabled?: boolean;
  onRecord: (accelerator: string) => void;
  onInvalid: (reason: "modifier" | "key") => void;
}) {
  const { t, lang } = useI18n();
  const [recording, setRecording] = useState(false);
  const ref = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const r = fromKeyEvent(e);
      if (r.kind === "pending") return;
      setRecording(false);
      if (r.kind === "done") props.onRecord(r.accelerator);
      else if (r.kind === "invalid") props.onInvalid(r.reason);
    };
    const stop = () => setRecording(false);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", stop);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", stop);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [recording]);

  return (
    <button
      ref={ref}
      type="button"
      className={`recorder${recording ? " recording" : ""}`}
      disabled={props.disabled}
      title={t("shortcut.record")}
      onClick={() => setRecording((r) => !r)}
    >
      {recording ? (
        <>
          <Keyboard size={14} /> {t("shortcut.recording")}
        </>
      ) : (
        <Kbd parts={displayParts(props.value, lang)} />
      )}
    </button>
  );
}
