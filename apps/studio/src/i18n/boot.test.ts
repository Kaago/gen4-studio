import { afterEach, describe, expect, it, vi } from "vitest";
import { readCss } from "../ui/cssSource";

// `vitest.setup.ts` initialises i18next before every test and i18next is one shared instance, so `isInitialized` could not tell
// whether `boot.ts` did its job. The tests replace `./index` and look at what `boot.ts` does with it, and at where `main.tsx` imports it.

afterEach(() => {
  vi.doUnmock("./index");
  vi.resetModules();
  localStorage.clear();
});

describe("i18n/boot", () => {
  it("initialises with the stored language as a side effect of being imported", async () => {
    const init = vi.fn();
    vi.resetModules();
    vi.doMock("./index", () => ({ init, readLanguage: () => "en" }));

    expect(init).not.toHaveBeenCalled();
    await import("./boot");

    expect(init).toHaveBeenCalledTimes(1);
    expect(init).toHaveBeenCalledWith("en");
  });

  it("falls back to the system language without a stored choice (real readLanguage)", async () => {
    const init = vi.fn();
    const actual = await vi.importActual<typeof import("./index")>("./index");
    vi.resetModules();
    vi.doMock("./index", () => ({ ...actual, init }));
    localStorage.removeItem("g4.lang");

    await import("./boot");

    expect(init).toHaveBeenCalledWith("system");
  });
});

describe("main.tsx", () => {
  it("imports ./i18n/boot before every other module", () => {
    // `readCss` is the tests' "read the file next to this test" helper, whatever the file type.
    const source = readCss("../main.tsx", import.meta.url);
    // Comments and blank lines aside, the first import statement is the boot module: the app's graph is evaluated after it.
    const imports = [...source.matchAll(/^import\s[^;]*?["']([^"']+)["'];/gm)].map((m) => m[1]);
    expect(imports[0]).toBe("./i18n/boot");
    expect(imports.indexOf("./App")).toBeGreaterThan(0);
  });
});
