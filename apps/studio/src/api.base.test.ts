import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
/** `tauri`: whether the page pretends to run inside the Tauri window. */
const env = vi.hoisted(() => ({ tauri: true }));
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => env.tauri }));

import { assignBase, baseStatus, checkBase, errorText, formatBytes, registerBaseRom, setLocale } from "./api";

describe("base ROM commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });

  it("base_commands_send_the_documented_arguments", async () => {
    await baseStatus();
    expect(invoke).toHaveBeenLastCalledWith("base_status");
    await checkBase("/roms/Sinnoh-Remix.nds");
    expect(invoke).toHaveBeenLastCalledWith("check_base", { nds: "/roms/Sinnoh-Remix.nds" });
    await assignBase("/roms/Sinnoh-Remix.nds");
    expect(invoke).toHaveBeenLastCalledWith("assign_base", { nds: "/roms/Sinnoh-Remix.nds" });
    await registerBaseRom("/elsewhere/Platin.nds");
    expect(invoke).toHaveBeenLastCalledWith("register_base_rom", { path: "/elsewhere/Platin.nds" });

    const status = { state: "unassigned", suggestion: "/roms/Sinnoh-Remix.nds" };
    invoke.mockResolvedValueOnce(status);
    await expect(baseStatus()).resolves.toEqual(status);
    // A found base carries the name the patch gets, from the backend.
    const found = { state: "found", mark: {}, label: "Platin DE (Testprojekt)", path: "/roms/X.nds", source: "known", patch_name: "Sinnoh-Remix-CPUD" };
    invoke.mockResolvedValueOnce(found);
    await expect(baseStatus()).resolves.toMatchObject({ patch_name: "Sinnoh-Remix-CPUD" });
  });

  it("format_bytes_uses_byte_kib_and_mib_with_a_comma", () => {
    expect(formatBytes(0)).toBe("0 Byte");
    expect(formatBytes(397)).toBe("397 Byte");
    expect(formatBytes(1023)).toBe("1023 Byte");
    expect(formatBytes(1024)).toBe("1,0 KiB");
    expect(formatBytes(4813)).toBe("4,7 KiB");
    expect(formatBytes(1_258_291)).toBe("1,2 MiB");
    // Rounding never shows „1024,0 KiB“.
    expect(formatBytes(1_048_575)).toBe("1,0 MiB");
  });

  it("a_base_missing_error_reads_as_its_message", () => {
    expect(errorText({ code: "base_missing", message: "Diesem Projekt ist keine Basis-ROM zugeordnet.", asset: null, problems: [] })).toBe(
      "Diesem Projekt ist keine Basis-ROM zugeordnet.",
    );
  });
});

describe("set_locale", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });
  afterEach(() => {
    vi.unstubAllEnvs();
    env.tauri = true;
  });

  it("inside Tauri tells the backend the language", async () => {
    await setLocale("en");
    expect(invoke).toHaveBeenCalledWith("set_locale", { lang: "en" });
  });

  it("in the browser during development tells the dev bridge, a separate process that would otherwise keep G4_LANG", async () => {
    env.tauri = false;
    vi.stubEnv("MODE", "development");
    await setLocale("de");
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("set_locale", { lang: "de" });
  });

  it("in the browser outside development (the tests, a build without a backend) calls nothing", async () => {
    env.tauri = false;
    vi.stubEnv("MODE", "test");
    await setLocale("de");
    expect(invoke).not.toHaveBeenCalled();
  });
});
