import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { setLocale } from "../api.base";
import { applyLanguage, detectLanguage, effectiveLanguage, init, LANGUAGE_KEY, matchesAnyLanguage, readLanguage } from "./index";

vi.mock("../api.base", () => ({ setLocale: vi.fn(() => Promise.resolve()) }));

const setLocaleMock = vi.mocked(setLocale);

/** A promise the test resolves by hand. */
function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

/** Follows a promise: `state()` tells whether it has settled by now. */
function track(p: Promise<unknown>): { state: () => "pending" | "fulfilled" | "rejected" } {
  let state: "pending" | "fulfilled" | "rejected" = "pending";
  p.then(
    () => {
      state = "fulfilled";
    },
    () => {
      state = "rejected";
    },
  );
  return { state: () => state };
}

/** Lets everything that can run without a timer run, so that a promise still `pending` afterwards is waiting for something outside. */
const settle = () => new Promise<void>((r) => setTimeout(r, 0));

function stubNavigatorLanguage(value: string): void {
  Object.defineProperty(navigator, "language", { value, configurable: true });
}

beforeEach(() => {
  localStorage.clear();
  setLocaleMock.mockReset();
  setLocaleMock.mockResolvedValue(undefined);
});

afterEach(async () => {
  // The suite runs in German (vitest.setup.ts); a test that switched the language puts it back.
  await i18next.changeLanguage("de");
  document.documentElement.lang = "";
  // The own property shadows jsdom's getter on the prototype; deleting it restores the original.
  Reflect.deleteProperty(navigator, "language");
});

describe("detectLanguage", () => {
  it.each(["de-AT", "DE", "de_CH", "de", "De-de"])("%s is German", (nav) => {
    expect(detectLanguage(nav)).toBe("de");
  });

  it.each(["en-GB", "fr", "", "dex-DE", "nl-DE", "xde"])("%j is English", (nav) => {
    expect(detectLanguage(nav)).toBe("en");
  });
});

describe("readLanguage", () => {
  it("answers system without an entry", () => {
    expect(readLanguage()).toBe("system");
  });

  it.each(["system", "de", "en"] as const)("reads the stored value %s", (value) => {
    localStorage.setItem(LANGUAGE_KEY, value);
    expect(readLanguage()).toBe(value);
  });

  it("answers system for a value that is no language", () => {
    localStorage.setItem("g4.lang", "xx");
    expect(readLanguage()).toBe("system");
  });
});

describe("effectiveLanguage", () => {
  it("follows the navigator language for system", () => {
    stubNavigatorLanguage("de-DE");
    expect(effectiveLanguage("system")).toBe("de");
    stubNavigatorLanguage("fr-FR");
    expect(effectiveLanguage("system")).toBe("en");
  });

  it("takes a fixed language as it is", () => {
    stubNavigatorLanguage("de-DE");
    expect(effectiveLanguage("en")).toBe("en");
    stubNavigatorLanguage("en-US");
    expect(effectiveLanguage("de")).toBe("de");
  });

  it("reads the stored language by default", () => {
    stubNavigatorLanguage("de-DE");
    localStorage.setItem(LANGUAGE_KEY, "en");
    expect(effectiveLanguage()).toBe("en");
  });
});

describe("init", () => {
  it("is idempotent: a second call does not initialise again", () => {
    expect(i18next.isInitialized).toBe(true);
    expect(i18next.language).toBe("de");
    init("en");
    expect(i18next.language).toBe("de");
  });

  it("finds the common catalog by default", () => {
    expect(i18next.t("appName")).toBe("Gen4 Studio");
  });
});

describe("applyLanguage", () => {
  it("stores the choice, switches i18next and the document, and tells the backend", async () => {
    await applyLanguage("en");
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe("en");
    expect(i18next.language).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    expect(setLocaleMock).toHaveBeenCalledWith("en");
  });

  it("tells the backend the effective language when the choice is system", async () => {
    stubNavigatorLanguage("de-AT");
    await applyLanguage("system");
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe("system");
    expect(document.documentElement.lang).toBe("de");
    expect(setLocaleMock).toHaveBeenCalledWith("de");
  });

  it("is pending until the backend has answered", async () => {
    const backend = deferred();
    setLocaleMock.mockReturnValue(backend.promise);

    const applied = track(applyLanguage("en"));
    // Everything before the backend call has happened (it is the call that is awaited), the promise is still open.
    await vi.waitFor(() => expect(setLocaleMock).toHaveBeenCalledWith("en"));
    await settle();
    expect(document.documentElement.lang).toBe("en");
    expect(applied.state()).toBe("pending");

    backend.resolve();
    await settle();
    expect(applied.state()).toBe("fulfilled");
  });

  it("still resolves when the backend refuses the locale", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    setLocaleMock.mockRejectedValue({ code: "error", message: "unknown command", asset: null });

    await expect(applyLanguage("en")).resolves.toBeUndefined();
    expect(document.documentElement.lang).toBe("en");
    expect(warn).toHaveBeenCalledTimes(1);
    warn.mockRestore();
  });
});

describe("plural keys", () => {
  it("plural keys pick one and other in both languages", async () => {
    // `common:filesChanged` is the probe: `_one` for 1, `_other` for 0 and for every other number.
    expect(i18next.t("filesChanged", { count: 1 })).toBe("1 Datei hat sich geändert");
    expect(i18next.t("filesChanged", { count: 0 })).toBe("0 Dateien haben sich geändert");
    expect(i18next.t("filesChanged", { count: 2 })).toBe("2 Dateien haben sich geändert");

    await i18next.changeLanguage("en");
    expect(i18next.t("filesChanged", { count: 1 })).toBe("1 file changed");
    expect(i18next.t("filesChanged", { count: 0 })).toBe("0 files changed");
    expect(i18next.t("filesChanged", { count: 2 })).toBe("2 files changed");
    // The suite runs in German; `afterEach` puts the language back.
  });
});

describe("matchesAnyLanguage", () => {
  it("matches the sentence of a key in either language, whichever is shown", async () => {
    for (const shown of ["de", "en"] as const) {
      await i18next.changeLanguage(shown);
      expect(matchesAnyLanguage("workbench:placeNames.unnamed", "Ohne Ortsnamen")).toBe(true);
      expect(matchesAnyLanguage("workbench:placeNames.unnamed", "No place name")).toBe(true);
      expect(matchesAnyLanguage("workbench:placeNames.unnamed", "No place name ")).toBe(false);
      expect(matchesAnyLanguage("workbench:placeNames.unnamed", null)).toBe(false);
    }
  });
});
