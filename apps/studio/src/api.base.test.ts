import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { assignBase, baseStatus, checkBase, createPatch, errorText, formatBytes, patchName, registerBaseRom } from "./api";

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
    await createPatch();
    expect(invoke).toHaveBeenLastCalledWith("create_patch");

    const status = { state: "unassigned", suggestion: "/roms/Sinnoh-Remix.nds" };
    invoke.mockResolvedValueOnce(status);
    await expect(baseStatus()).resolves.toEqual(status);
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

  it("patch_name_follows_the_kernel", () => {
    expect(patchName("/roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents", "CPUD")).toBe("Pokemon - Platin-Edition (Germany)-CPUD");
    expect(patchName("/p/Sinnoh-Remix/", "CPUD")).toBe("Sinnoh-Remix-CPUD");
    expect(patchName("C:\\Hacks\\Mein:Hack_DSPRE_contents", "CPUE")).toBe("Mein_Hack-CPUE");
    expect(patchName("/p/ .Rom. ", "CPUE")).toBe("Rom-CPUE");
    expect(patchName("/p/...", "CPUE")).toBe("Projekt-CPUE");
  });
});
