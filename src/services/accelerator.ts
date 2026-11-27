// Converts keyboard events into accelerators understood by the backend
// ("Ctrl+Shift+H"). The backend re-validates everything (shortcuts/mod.rs).

export interface KeyLike {
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "OSLeft",
  "OSRight",
]);

const CODE_NAMES: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Minus: "Minus",
  Equal: "Equal",
  Comma: "Comma",
  Period: "Period",
  Slash: "Slash",
  Backslash: "Backslash",
  Semicolon: "Semicolon",
  Quote: "Quote",
  BracketLeft: "BracketLeft",
  BracketRight: "BracketRight",
  Backquote: "Backquote",
  Pause: "Pause",
  PrintScreen: "PrintScreen",
  ScrollLock: "ScrollLock",
  NumpadAdd: "NumpadAdd",
  NumpadSubtract: "NumpadSubtract",
  NumpadMultiply: "NumpadMultiply",
  NumpadDivide: "NumpadDivide",
  NumpadDecimal: "NumpadDecimal",
  NumpadEnter: "NumpadEnter",
};

/** Physical key name, independent of the keyboard layout (AZERTY friendly). */
export function keyName(code: string): string | null {
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return m[1];
  m = /^Digit([0-9])$/.exec(code);
  if (m) return m[1];
  m = /^Numpad([0-9])$/.exec(code);
  if (m) return `Numpad${m[1]}`;
  m = /^F([0-9]{1,2})$/.exec(code);
  if (m && Number(m[1]) >= 1 && Number(m[1]) <= 24) return `F${m[1]}`;
  return CODE_NAMES[code] ?? null;
}

export type RecordResult =
  | { kind: "pending" }
  | { kind: "cancel" }
  | { kind: "invalid"; reason: "modifier" | "key" }
  | { kind: "done"; accelerator: string };

export function fromKeyEvent(e: KeyLike): RecordResult {
  if (e.code === "Escape" && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) return { kind: "cancel" };
  if (MODIFIER_CODES.has(e.code)) return { kind: "pending" };
  const key = keyName(e.code);
  if (!key) return { kind: "invalid", reason: "key" };
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  if (parts.length === 0) return { kind: "invalid", reason: "modifier" };
  if (parts.length === 1 && parts[0] === "Shift" && key.length === 1) return { kind: "invalid", reason: "modifier" };
  parts.push(key);
  return { kind: "done", accelerator: parts.join("+") };
}

/** Labels shown to the user ("Super" is the Windows key). */
export function displayParts(accelerator: string, lang: "en" | "fr" = "en"): string[] {
  return accelerator.split("+").map((p) => {
    if (p === "Super") return "Win";
    if (p === "Shift" && lang === "fr") return "Maj";
    if (p === "Up") return "↑";
    if (p === "Down") return "↓";
    if (p === "Left") return "←";
    if (p === "Right") return "→";
    return p;
  });
}
