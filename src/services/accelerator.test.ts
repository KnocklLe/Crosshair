import { describe, expect, it } from "vitest";
import { displayParts, fromKeyEvent, keyName, type KeyLike } from "./accelerator";

const ev = (code: string, mods: Partial<KeyLike> = {}): KeyLike => ({
  key: "",
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("shortcut capture", () => {
  it("builds the default shortcuts", () => {
    expect(fromKeyEvent(ev("KeyH", { ctrlKey: true, shiftKey: true }))).toEqual({ kind: "done", accelerator: "Ctrl+Shift+H" });
    expect(fromKeyEvent(ev("KeyD", { ctrlKey: true, shiftKey: true }))).toEqual({ kind: "done", accelerator: "Ctrl+Shift+D" });
  });

  it("uses the physical key, whatever the layout (AZERTY)", () => {
    // On AZERTY the physical "Q" key types "a": the accelerator must stay Q.
    expect(fromKeyEvent({ ...ev("KeyQ", { altKey: true }), key: "a" })).toEqual({ kind: "done", accelerator: "Alt+Q" });
  });

  it("waits while only modifiers are pressed", () => {
    expect(fromKeyEvent(ev("ControlLeft", { ctrlKey: true }))).toEqual({ kind: "pending" });
    expect(fromKeyEvent(ev("ShiftRight", { shiftKey: true }))).toEqual({ kind: "pending" });
  });

  it("requires a modifier", () => {
    expect(fromKeyEvent(ev("KeyA"))).toEqual({ kind: "invalid", reason: "modifier" });
    expect(fromKeyEvent(ev("KeyA", { shiftKey: true }))).toEqual({ kind: "invalid", reason: "modifier" });
    expect(fromKeyEvent(ev("F2", { shiftKey: true }))).toEqual({ kind: "done", accelerator: "Shift+F2" });
  });

  it("cancels on Escape and rejects unknown keys", () => {
    expect(fromKeyEvent(ev("Escape"))).toEqual({ kind: "cancel" });
    expect(fromKeyEvent(ev("IntlBackslash", { ctrlKey: true }))).toEqual({ kind: "invalid", reason: "key" });
  });

  it("orders modifiers canonically", () => {
    expect(fromKeyEvent(ev("ArrowUp", { metaKey: true, altKey: true, ctrlKey: true }))).toEqual({
      kind: "done",
      accelerator: "Ctrl+Alt+Super+Up",
    });
  });

  it("maps key codes", () => {
    expect(keyName("Digit7")).toBe("7");
    expect(keyName("Numpad3")).toBe("Numpad3");
    expect(keyName("F24")).toBe("F24");
    expect(keyName("F25")).toBeNull();
    expect(keyName("PageDown")).toBe("PageDown");
  });

  it("displays localized labels", () => {
    expect(displayParts("Ctrl+Shift+H", "fr")).toEqual(["Ctrl", "Maj", "H"]);
    expect(displayParts("Alt+Super+Left")).toEqual(["Alt", "Win", "←"]);
  });
});
