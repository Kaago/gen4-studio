import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => true }));

import { dialogOf, dialogsOfHeader, editTextArchive, splitMessage } from "./api";
import type { Dialog, HeaderDialogs, SplitMessageReport } from "./api";

describe("dialog commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(null);
  });

  it("dialog_commands_send_the_documented_arguments", async () => {
    const dialog: Dialog = {
      source: { kind: "local", file: 0, slot: 3, line: 1 },
      messages: [{ archive: 433, index: 5, text: "Hallo", command: "Message", line: 2, col: 13, branch: null, shared_with: 1, split_refusal: null }],
      followed: [],
    };
    invoke.mockResolvedValueOnce(dialog);
    await expect(dialogOf(1, 0, "object", 0)).resolves.toBe(dialog);
    expect(invoke).toHaveBeenLastCalledWith("dialog_of", { header: 1, eventFile: 0, kind: "object", index: 0 });
    await dialogOf(1, 0, "bg", 2);
    expect(invoke).toHaveBeenLastCalledWith("dialog_of", { header: 1, eventFile: 0, kind: "bg", index: 2 });

    const report: SplitMessageReport = { archive: 3, old_index: 5, new_index: 10, change: 7, lines: [2, 4] };
    invoke.mockResolvedValueOnce(report);
    await expect(splitMessage(1, 0, 0, 3, 5)).resolves.toBe(report);
    expect(invoke).toHaveBeenLastCalledWith("split_message", { header: 1, eventFile: 0, index: 0, archive: 3, message: 5 });

    // The dialog editor names its step; without a label the backend names it after the edit.
    await editTextArchive(433, { Set: { index: 5, text: "Neu" } }, "Dialog geändert");
    expect(invoke).toHaveBeenLastCalledWith("edit_text_archive", { index: 433, edit: { Set: { index: 5, text: "Neu" } }, label: "Dialog geändert" });
    await editTextArchive(433, "Append");
    expect(invoke).toHaveBeenLastCalledWith("edit_text_archive", { index: 433, edit: "Append" });
  });

  it("dialogs_of_header_sends_the_header_and_answers_the_figures", async () => {
    const place: HeaderDialogs = {
      header: 3,
      event_file: 2,
      archive: 433,
      figures: [
        {
          kind: "object", index: 0, local_id: 4, graphics_id: 91, trainer: null,
          source: { kind: "local", file: 0, slot: 1, line: 1 },
          first_text: String.raw`A\nB`, messages: 2, pages: 3, message_ids: [2, 3],
        },
        { kind: "object", index: 1, local_id: 1, graphics_id: 0, trainer: 6, source: { kind: "trainer", id: 6 }, first_text: null, messages: 0, pages: 0, message_ids: [] },
        { kind: "bg", index: 0, local_id: null, graphics_id: null, trainer: null, source: { kind: "none" }, first_text: null, messages: 0, pages: 0, message_ids: [] },
      ],
      ms: 12,
    };
    invoke.mockResolvedValueOnce(place);
    await expect(dialogsOfHeader(3)).resolves.toBe(place);
    expect(invoke).toHaveBeenLastCalledWith("dialogs_of_header", { header: 3 });
  });

  it("dialog_sources_name_every_kind_the_kernel_answers", () => {
    const sources: Dialog["source"][] = [
      { kind: "local", file: 0, slot: 3, line: 1 },
      { kind: "common", script: 2010 },
      { kind: "trainer", id: 12 },
      { kind: "none" },
      { kind: "unreadable", reason: "Erst Scripts übernehmen." },
    ];
    expect(sources.map((s) => s.kind)).toEqual(["local", "common", "trainer", "none", "unreadable"]);
  });
});
