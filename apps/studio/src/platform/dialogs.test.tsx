// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { open as tauriOpen, save as tauriSave, ask as tauriAsk } from "@tauri-apps/plugin-dialog";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ask, openPath, projectFromUrl, savePath } from "./dialogs";
import { WebDialogHost } from "./WebDialogHost";

vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("in-page dialogs (browser mode)", () => {
  let container: HTMLElement;
  let root: Root;

  beforeEach(async () => {
    localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<WebDialogHost />));
  });
  afterEach(() => {
    vi.restoreAllMocks();
    act(() => root.unmount());
    container.remove();
  });

  // The dialog is a Base UI dialog in a portal: it sits in the body, not in the container.
  const dialog = () => document.querySelector<HTMLElement>('[role="dialog"]');
  const button = (label: string) => {
    const b = [...(dialog()?.querySelectorAll("button") ?? [])].find((x) => x.textContent === label);
    if (!b) throw new Error(`no button ${label}`);
    return b;
  };
  const click = (label: string) => act(async () => button(label).click());
  const field = () => document.querySelector<HTMLInputElement>("[role=dialog] input")!;
  const type = (value: string) =>
    act(async () => {
      const input = field();
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
  const escape = () => act(async () => void document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));

  it("ask_resolves_true_on_ja", async () => {
    const answer = ask("Verwerfen?", { kind: "warning" });
    await act(async () => {});
    expect(dialog()?.textContent).toContain("Verwerfen?");
    await click("Ja");
    await expect(answer).resolves.toBe(true);
    expect(dialog()).toBeNull();
  });

  it("ask_resolves_false_on_nein", async () => {
    const answer = ask("Verwerfen?");
    await act(async () => {});
    await click("Nein");
    await expect(answer).resolves.toBe(false);
  });

  it("ask_resolves_false_on_escape", async () => {
    const answer = ask("Verwerfen?");
    await act(async () => {});
    await escape();
    await expect(answer).resolves.toBe(false);
    expect(dialog()).toBeNull();
  });

  it("open_path_returns_the_typed_path", async () => {
    const answer = openPath({ directory: true });
    await act(async () => {});
    await type("/Users/x/pt-gate");
    await click("OK");
    await expect(answer).resolves.toBe("/Users/x/pt-gate");
  });

  it("save_path_prefills_the_default", async () => {
    const answer = savePath({ defaultPath: "/p/out.nds" });
    await act(async () => {});
    expect(field().value).toBe("/p/out.nds");
    await click("OK");
    await expect(answer).resolves.toBe("/p/out.nds");
  });

  it("path_dialog_cancels_on_abbrechen_and_on_escape_even_with_text_in_the_field", async () => {
    const first = openPath({ directory: true });
    await act(async () => {});
    await type("/x");
    await click("Abbrechen");
    await expect(first).resolves.toBeNull();
    const second = savePath({ defaultPath: "/p/out.nds" });
    await act(async () => {});
    field().focus();
    await escape();
    await expect(second).resolves.toBeNull();
  });

  it("empty_path_is_null", async () => {
    const answer = openPath({ directory: true });
    await act(async () => {});
    await type("   ");
    await click("OK");
    await expect(answer).resolves.toBeNull();
  });

  it("remembers_the_last_path", async () => {
    const first = openPath({ directory: true });
    await act(async () => {});
    await type("/Users/x/pt-gate");
    await click("OK");
    await first;
    const second = openPath({ directory: true });
    await act(async () => {});
    expect(field().value).toBe("/Users/x/pt-gate");
    await click("Abbrechen");
    await second;
  });

  it("local_storage_failure_is_ignored", async () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const answer = openPath({ directory: true });
    await act(async () => {});
    expect(field().value).toBe("");
    await type("/p");
    await click("OK");
    await expect(answer).resolves.toBe("/p");
  });

  it("shows_a_second_request_after_the_first_is_answered", async () => {
    const first = ask("Eins?");
    const second = ask("Zwei?");
    await act(async () => {});
    expect(dialog()?.textContent).toContain("Eins?");
    await click("Ja");
    await first;
    expect(dialog()?.textContent).toContain("Zwei?");
    await click("Nein");
    await expect(second).resolves.toBe(false);
  });

  it("ask_is_a_dialog_named_rueckfrage_with_the_message_and_nein_before_ja", async () => {
    void ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" });
    await act(async () => {});
    const el = dialog()!;
    expect(document.getElementById(el.getAttribute("aria-labelledby")!)?.textContent).toBe("Rückfrage");
    expect(el.textContent).toContain("Ungespeicherte Änderungen verwerfen?");
    const labels = [...el.querySelectorAll("button")].map((b) => b.textContent || b.getAttribute("aria-label"));
    expect(labels.slice(0, 2)).toEqual(["Nein", "Ja"]);
    await click("Nein");
  });

  it("path_dialog_is_named_by_its_title_and_has_a_field_named_pfad", async () => {
    const answer = openPath({ directory: false, title: "Emulator wählen" });
    await act(async () => {});
    const el = dialog()!;
    expect(document.getElementById(el.getAttribute("aria-labelledby")!)?.textContent).toBe("Emulator wählen");
    expect(field().getAttribute("aria-label")).toBe("Pfad");
    await click("Abbrechen");
    await expect(answer).resolves.toBeNull();
  });

  it("enter_in_the_path_field_takes_the_typed_path", async () => {
    const answer = openPath({ directory: true });
    await act(async () => {});
    await type("/Users/x/pt-enter");
    field().focus();
    await act(async () => void field().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
    await expect(answer).resolves.toBe("/Users/x/pt-enter");
    expect(dialog()).toBeNull();
  });

  it("blocks_shortcuts_behind_it_with_a_modal_dialog", async () => {
    void ask("Verwerfen?");
    await act(async () => {});
    expect(dialog()?.getAttribute("aria-modal")).toBe("true");
    await click("Nein");
  });
});

describe("project_from_url", () => {
  it("project_from_url_decodes_the_path", () => {
    expect(projectFromUrl("?project=%2FUsers%2Fx%2Fpt-gate")).toBe("/Users/x/pt-gate");
    expect(projectFromUrl("?project=/a/b")).toBe("/a/b");
    expect(projectFromUrl("")).toBeNull();
    expect(projectFromUrl("?other=1")).toBeNull();
    expect(projectFromUrl("?project=")).toBeNull();
  });
});

describe("dialogs inside Tauri", () => {
  const w = window as unknown as { __TAURI_INTERNALS__?: unknown };
  beforeEach(() => {
    vi.resetAllMocks();
    w.__TAURI_INTERNALS__ = {};
  });
  afterEach(() => {
    delete w.__TAURI_INTERNALS__;
  });

  it("forwards ask to the native dialog", async () => {
    vi.mocked(tauriAsk).mockResolvedValue(true);
    await expect(ask("Sicher?", { kind: "warning" })).resolves.toBe(true);
    expect(tauriAsk).toHaveBeenCalledWith("Sicher?", { kind: "warning" });
  });

  it("forwards openPath with directory and title, and maps a cancelled pick to null", async () => {
    vi.mocked(tauriOpen).mockResolvedValueOnce("/Applications/melonDS.app").mockResolvedValueOnce(null);
    await expect(openPath({ directory: false, title: "Emulator wählen (z. B. melonDS)" })).resolves.toBe("/Applications/melonDS.app");
    expect(tauriOpen).toHaveBeenCalledWith({ directory: false, multiple: false, title: "Emulator wählen (z. B. melonDS)" });
    await expect(openPath({ directory: true })).resolves.toBeNull();
  });

  it("keeps the .nds filter for a ROM and none for other save paths", async () => {
    vi.mocked(tauriSave).mockResolvedValue("/p/out.nds");
    await expect(savePath({ defaultPath: "/p/out.nds" })).resolves.toBe("/p/out.nds");
    expect(tauriSave).toHaveBeenCalledWith({ defaultPath: "/p/out.nds", filters: [{ name: "ROM", extensions: ["nds"] }] });
    await savePath({ defaultPath: "/p/notes.txt" });
    expect(tauriSave).toHaveBeenLastCalledWith({ defaultPath: "/p/notes.txt" });
  });
});
