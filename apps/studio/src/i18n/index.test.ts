import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { setLocale, systemLanguage } from "../api.base";
import {
  applyLanguage,
  detectLanguage,
  effectiveLanguage,
  forgetSystemLanguage,
  init,
  LANGUAGE_KEY,
  matchesAnyLanguage,
  readLanguage,
  resolveSystemLanguage,
  SET_LOCALE_TIMEOUT_MS,
  startLanguage,
  SYSTEM_LANGUAGE_TIMEOUT_MS,
} from "./index";

// `systemLanguage` is the Tauri side's `system_language`: `null` stands for the browser (nothing to ask), a string for the OS locale.
vi.mock("../api.base", () => ({ setLocale: vi.fn(() => Promise.resolve()), systemLanguage: vi.fn(() => Promise.resolve(null)) }));

const setLocaleMock = vi.mocked(setLocale);
const systemLanguageMock = vi.mocked(systemLanguage);

/** A promise the test resolves by hand. */
function deferred<T = void>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
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
  systemLanguageMock.mockReset();
  systemLanguageMock.mockResolvedValue(null);
  forgetSystemLanguage();
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
  it("is idempotent: a second call does not initialize again", () => {
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

  it("tells the backend before i18next switches: set_locale sees the old language", async () => {
    let shownWhenTold: string | undefined;
    setLocaleMock.mockImplementation(async () => {
      shownWhenTold = i18next.language;
    });

    await applyLanguage("en");

    expect(setLocaleMock).toHaveBeenCalledWith("en");
    expect(shownWhenTold).toBe("de");
    expect(i18next.language).toBe("en");
  });

  it("is pending until the backend has answered, and shows nothing new before", async () => {
    const backend = deferred();
    setLocaleMock.mockReturnValue(backend.promise);

    const applied = track(applyLanguage("en"));
    await vi.waitFor(() => expect(setLocaleMock).toHaveBeenCalledWith("en"));
    await settle();
    // The choice is remembered at once; the language shown waits for the backend, so no text is built in a language it does not speak yet.
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe("en");
    expect(i18next.language).toBe("de");
    expect(document.documentElement.lang).not.toBe("en");
    expect(applied.state()).toBe("pending");

    backend.resolve();
    await settle();
    expect(applied.state()).toBe("fulfilled");
    expect(i18next.language).toBe("en");
    expect(document.documentElement.lang).toBe("en");
  });

  it("waits at most SET_LOCALE_TIMEOUT_MS for the backend, then switches anyway and warns once", async () => {
    vi.useFakeTimers();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      expect(SET_LOCALE_TIMEOUT_MS).toBe(3000);
      // The backend never answers.
      setLocaleMock.mockReturnValue(new Promise<void>(() => {}));
      const applied = track(applyLanguage("en"));
      await vi.advanceTimersByTimeAsync(SET_LOCALE_TIMEOUT_MS - 1);
      expect(applied.state()).toBe("pending");
      expect(i18next.language).toBe("de");
      expect(warn).not.toHaveBeenCalled();

      await vi.advanceTimersByTimeAsync(1);
      expect(applied.state()).toBe("fulfilled");
      expect(i18next.language).toBe("en");
      expect(document.documentElement.lang).toBe("en");
      expect(warn).toHaveBeenCalledTimes(1);
    } finally {
      warn.mockRestore();
      vi.useRealTimers();
    }
  });

  it("lets the latest choice win when an earlier one is still waiting for the backend", async () => {
    const first = deferred();
    setLocaleMock.mockReturnValueOnce(first.promise);

    const early = track(applyLanguage("en"));
    await vi.waitFor(() => expect(setLocaleMock).toHaveBeenCalledTimes(1));
    // German was the language shown; the second choice is answered at once.
    await applyLanguage("de");
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe("de");

    // The first backend answers late: it must not switch the app back to English.
    first.resolve();
    await settle();
    expect(early.state()).toBe("fulfilled");
    expect(i18next.language).toBe("de");
    expect(document.documentElement.lang).toBe("de");
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

describe("resolveSystemLanguage", () => {
  it.each([
    ["de-AT", "de"],
    ["de_DE.UTF-8", "de"],
    ["fr-FR", "en"],
    ["en-US", "en"],
  ] as const)("inside Tauri the OS locale %s gives %s, whatever the navigator says", async (os, expected) => {
    stubNavigatorLanguage(expected === "de" ? "en-US" : "de-DE");
    systemLanguageMock.mockResolvedValue(os);
    await expect(resolveSystemLanguage()).resolves.toBe(expected);
    expect(effectiveLanguage("system")).toBe(expected);
  });

  it("asks the backend once and keeps the answer, also when the navigator changes", async () => {
    stubNavigatorLanguage("en-US");
    systemLanguageMock.mockResolvedValue("de-DE");
    await Promise.all([resolveSystemLanguage(), resolveSystemLanguage()]);
    await resolveSystemLanguage();
    expect(systemLanguageMock).toHaveBeenCalledTimes(1);
    stubNavigatorLanguage("fr-FR");
    expect(effectiveLanguage("system")).toBe("de");
    // A fixed choice stays what it is.
    expect(effectiveLanguage("en")).toBe("en");
  });

  it("falls back to the navigator when the backend fails", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    stubNavigatorLanguage("de-AT");
    systemLanguageMock.mockRejectedValue({ code: "error", message: "unknown command system_language", asset: null });
    await expect(resolveSystemLanguage()).resolves.toBe("de");
    expect(warn).toHaveBeenCalledTimes(1);
    // Nothing is kept: the navigator goes on deciding.
    stubNavigatorLanguage("en-GB");
    expect(effectiveLanguage("system")).toBe("en");
    warn.mockRestore();
  });

  it("lets the navigator decide in the browser, where there is nothing to ask", async () => {
    stubNavigatorLanguage("de-CH");
    await expect(resolveSystemLanguage()).resolves.toBe("de");
    stubNavigatorLanguage("en-US");
    expect(effectiveLanguage("system")).toBe("en");
  });

  it("does not wait longer than the timeout and drops a late answer", async () => {
    vi.useFakeTimers();
    try {
      stubNavigatorLanguage("en-US");
      const late = deferred<string | null>();
      systemLanguageMock.mockReturnValue(late.promise);
      const resolved = track(resolveSystemLanguage());
      await vi.advanceTimersByTimeAsync(SYSTEM_LANGUAGE_TIMEOUT_MS - 1);
      expect(resolved.state()).toBe("pending");
      await vi.advanceTimersByTimeAsync(1);
      expect(resolved.state()).toBe("fulfilled");
      await expect(resolveSystemLanguage()).resolves.toBe("en");
      late.resolve("de-DE");
      await vi.advanceTimersByTimeAsync(0);
      expect(effectiveLanguage("system")).toBe("en");
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("startLanguage", () => {
  it("with System, waits for the backend's system language before it switches, then shows that one", async () => {
    // The suite shows German and the navigator says German; the OS (through the backend) says French, so English.
    stubNavigatorLanguage("de-DE");
    const os = deferred<string | null>();
    systemLanguageMock.mockReturnValue(os.promise);

    const started = track(startLanguage());
    await settle();
    expect(started.state()).toBe("pending");
    expect(i18next.language).toBe("de");
    expect(setLocaleMock).not.toHaveBeenCalled();

    os.resolve("fr-FR");
    await vi.waitFor(() => expect(started.state()).toBe("fulfilled"));
    expect(i18next.language).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    expect(setLocaleMock).toHaveBeenCalledWith("en");
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe("system");
  });

  it("with a fixed choice, does not wait for the backend but asks it for a later switch to System", async () => {
    localStorage.setItem(LANGUAGE_KEY, "en");
    stubNavigatorLanguage("en-US");
    const os = deferred<string | null>();
    systemLanguageMock.mockReturnValue(os.promise);

    await startLanguage();
    expect(i18next.language).toBe("en");
    expect(systemLanguageMock).toHaveBeenCalledTimes(1);

    os.resolve("de-AT");
    await settle();
    // The switch in Einstellungen finds the answer at once and asks no second time.
    await applyLanguage("system");
    expect(i18next.language).toBe("de");
    expect(setLocaleMock).toHaveBeenLastCalledWith("de");
    expect(systemLanguageMock).toHaveBeenCalledTimes(1);
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
