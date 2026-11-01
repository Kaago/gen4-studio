import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { errorText, flagUsage, freeFlag, isCmdError, listFlags, nameFlag } from "./api";

describe("flag commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });

  it("flag_commands_send_the_documented_arguments", async () => {
    await listFlags("flag");
    expect(invoke).toHaveBeenLastCalledWith("list_flags", { kind: "flag" });
    await listFlags("var");
    expect(invoke).toHaveBeenLastCalledWith("list_flags", { kind: "var" });
    await flagUsage("flag", 148);
    expect(invoke).toHaveBeenLastCalledWith("flag_usage", { kind: "flag", number: 148 });
    await freeFlag("var");
    expect(invoke).toHaveBeenLastCalledWith("free_flag", { kind: "var" });
    // Only what is asked for goes along: a description alone, or a name alone.
    await nameFlag({ kind: "flag", number: 148, id: "FLAG_HAT_MEDAILLE" });
    expect(invoke).toHaveBeenLastCalledWith("name_flag", { kind: "flag", number: 148, id: "FLAG_HAT_MEDAILLE" });
    await nameFlag({ kind: "var", number: 16432, description: "Zählt die Besuche." });
    expect(invoke).toHaveBeenLastCalledWith("name_flag", { kind: "var", number: 16432, description: "Zählt die Besuche." });
    await nameFlag({ kind: "flag", number: 148, id: "FLAG_X", description: "Satz." });
    expect(invoke).toHaveBeenLastCalledWith("name_flag", { kind: "flag", number: 148, id: "FLAG_X", description: "Satz." });
  });

  it("flag_refusals_are_command_errors", () => {
    for (const code of ["scripts_not_adopted", "pool_empty", "invalid_id", "taken", "refused"]) {
      expect(isCmdError({ code, message: "Satz.", asset: null })).toBe(true);
    }
    // So the sentence of the kernel reaches the user, not "[object Object]".
    expect(errorText({ code: "pool_empty", message: "Kein freies Flag im allgemeinen Bereich.", asset: null, problems: [] })).toBe("Kein freies Flag im allgemeinen Bereich.");
  });
});
