import { describe, expect, it } from "vitest";
import { en } from "./en";
import { fr } from "./fr";
import { interpolate, resolveLanguage, setLanguage, translate } from "./index";

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("translations", () => {
  it("French covers exactly the English keys", () => {
    expect(Object.keys(fr).sort()).toEqual(Object.keys(en).sort());
  });

  it("no string is empty and placeholders match", () => {
    for (const key of Object.keys(en) as (keyof typeof en)[]) {
      expect(en[key].trim(), key).not.toBe("");
      expect(fr[key].trim(), key).not.toBe("");
      expect(placeholders(fr[key]), key).toEqual(placeholders(en[key]));
    }
  });

  it("interpolates variables and keeps unknown ones", () => {
    expect(interpolate("Profile: {name}", { name: "Pro" })).toBe("Profile: Pro");
    expect(interpolate("{a} {b}", { a: 1 })).toBe("1 {b}");
  });

  it("resolves the language", () => {
    expect(resolveLanguage("auto", "fr-FR")).toBe("fr");
    expect(resolveLanguage("auto", "en-US")).toBe("en");
    expect(resolveLanguage("en", "fr-FR")).toBe("en");
    expect(resolveLanguage("fr", "de-DE")).toBe("fr");
  });

  it("translates with the current language", () => {
    setLanguage("fr");
    expect(translate("nav.settings")).toBe("Paramètres");
    setLanguage("en");
    expect(translate("nav.settings")).toBe("Settings");
    expect(translate("warning.unknown_code")).toBe("warning.unknown_code");
  });
});
