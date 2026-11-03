// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { freeFlag, listFlags } from "../api";
import type { FlagListDto, ProjectInfo } from "../api";
import { FLAGS, flagEntry, flagList } from "../flags/flagTesting";
import { invalidateFlagLists } from "../flags/useFlagList";
import { typeInto } from "../ui/primitives/testUtils";
import { NewStoryDialog } from "./NewStoryDialog";
import type { NewStoryRequest } from "./NewStoryDialog";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), listFlags: vi.fn(), freeFlag: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const project: ProjectInfo = { root: "/p", game_code: "CPUD", family: "Pt", language: "de", header_count: 1 };
const VARS: FlagListDto = flagList("var", [
  flagEntry(0x4040, { name: "VAR_UNK_0x4040", own_id: "VAR_RIVALIN" }),
  flagEntry(0x406c, { name: "VAR_UNUSED_0x406C", free: true }),
  flagEntry(0x4071, { free: true }),
]);

function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
}

describe("NewStoryDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onCreate = vi.fn<(req: NewStoryRequest) => Promise<void>>();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    invalidateFlagLists();
    vi.mocked(listFlags).mockReset().mockImplementation(async (kind) => (kind === "flag" ? FLAGS : VARS));
    vi.mocked(freeFlag).mockReset().mockResolvedValue({ kind: "var", number: 0x406c, name: "VAR_UNUSED_0x406C", pool_free: 20, scripts_indexed: true });
    onClose.mockReset();
    onCreate.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (open = true) => {
    await act(async () => root.render(<NewStoryDialog open={open} onClose={onClose} onCreate={onCreate} project={project} revision={0} />));
    await flush();
  };
  const dialog = () => document.querySelector<HTMLElement>("[data-new-story-dialog]");
  const field = (label: string) => dialog()!.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`)!;
  const button = (text: string) => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")];
  /** Picks the variable or flag with the hex number `tech` in the open field's picker. */
  const pick = async (tech: string) => {
    await click(dialog()!.querySelector<HTMLButtonElement>('[aria-label="Fortschritt wählen…"]'));
    await flush();
    await click(options().find((o) => o.textContent?.includes(tech)));
    await flush();
  };
  const kindButton = (label: string) => [...dialog()!.querySelectorAll<HTMLElement>("[data-progress-kind] button")].find((b) => b.textContent === label)!;
  const pool = () => dialog()!.querySelector<HTMLElement>("[data-pool-free]");

  it("new_story_dialog_takes_the_free_variable_and_calls_the_recipe", async () => {
    await show();
    // Opening takes the pool's smallest free variable into the field and says how many are free.
    expect(freeFlag).toHaveBeenCalledWith("var");
    expect(dialog()!.querySelector("[data-flag-field]")?.textContent).toContain("VAR_UNUSED_0x406C");
    expect(pool()?.textContent).toBe("Noch 20 Variablen frei");
    typeInto(field("Name"), "  Rivalin  ");
    typeInto(field("Notizen"), "Die Rivalin begleitet den Weg.");
    // „Neue Geschichte“ is the recipe: name, the number of the variable (the recipe names it VAR_STORY_…) and the notes.
    const pending = deferred<void>();
    onCreate.mockImplementationOnce(() => pending.promise);
    await click(button("Geschichte anlegen"));
    expect(onCreate).toHaveBeenCalledWith({ via: "recipe", input: { name: "Rivalin", var: 0x406c, notes: "Die Rivalin begleitet den Weg." } });
    // It takes seconds: locked meanwhile, with the busy line.
    expect(button("Geschichte anlegen")?.disabled).toBe(true);
    expect(dialog()?.textContent).toContain("Wird angelegt …");
    await act(async () => pending.resolve());
    await flush();
    expect(onClose).toHaveBeenCalledOnce();
    // The field stays changeable; a flag has no pool line, and back on Variable the free one is there again.
    await show(false);
    await show();
    expect(freeFlag).toHaveBeenCalledTimes(2);
    typeInto(field("Name"), "Rivalin");
    await pick("0x4071");
    await click(kindButton("Flag"));
    expect(pool()).toBeNull();
    await click(kindButton("Variable"));
    expect(dialog()!.querySelector("[data-flag-field]")?.textContent).toContain("VAR_UNUSED_0x406C");
    expect(pool()?.textContent).toBe("Noch 20 Variablen frei");
    vi.mocked(freeFlag).mockResolvedValueOnce({ kind: "var", number: 0x4071, name: null, pool_free: 1, scripts_indexed: true });
    await show(false);
    await show();
    expect(pool()?.textContent).toBe("Noch 1 Variable frei");
  });

  it("without_a_free_variable_the_dialog_says_so_and_locks_the_button", async () => {
    vi.mocked(freeFlag).mockRejectedValue({ code: "pool_empty", message: "Keine freie Variable im allgemeinen Bereich.", asset: null });
    await show();
    expect(pool()?.textContent).toBe("Keine freie Variable im allgemeinen Bereich.");
    expect(dialog()!.querySelector("[data-flag-field]")?.textContent).toContain("Keins");
    typeInto(field("Name"), "Rivalin");
    await flush();
    expect(button("Geschichte anlegen")?.disabled).toBe(true);
    // A variable picked by hand does not change it: the recipe takes one of the pool.
    await pick("0x4040");
    expect(button("Geschichte anlegen")?.disabled).toBe(true);
    // A flag story does not need the pool.
    await click(kindButton("Flag"));
    expect(pool()).toBeNull();
    await pick("0x0173");
    expect(button("Geschichte anlegen")?.disabled).toBe(false);
    await click(button("Geschichte anlegen"));
    await flush();
    expect(onCreate).toHaveBeenCalledWith({ via: "story", req: { name: "Rivalin", progress: { flag: "FLAG_0371" } } });
  });

  it("create_is_locked_without_name_or_progress", async () => {
    await show();
    expect(dialog()?.getAttribute("role")).toBe("dialog");
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Neue Geschichte");
    // Exactly one primary button: „Geschichte anlegen“; „Abbrechen“ beside it.
    expect([...dialog()!.querySelectorAll("button")].map((b) => b.textContent)).toEqual(expect.arrayContaining(["Abbrechen", "Geschichte anlegen"]));
    // Variable first, with the pool's free one.
    expect(dialog()!.querySelector("[data-progress-kind]")?.getAttribute("data-progress-kind")).toBe("var");
    expect(dialog()!.querySelector("[data-flag-field]")?.textContent).toContain("VAR_UNUSED_0x406C");
    expect(button("Geschichte anlegen")?.disabled).toBe(true);
    // A name makes it ready (the name only of spaces is none).
    typeInto(field("Name"), "Rivalin");
    await flush();
    expect(button("Geschichte anlegen")?.disabled).toBe(false);
    typeInto(field("Name"), "   ");
    await flush();
    expect(button("Geschichte anlegen")?.disabled).toBe(true);
    typeInto(field("Name"), "Rivalin");
    await flush();
    // Another kind starts at none.
    await click(kindButton("Flag"));
    expect(dialog()!.querySelector("[data-progress-kind]")?.getAttribute("data-progress-kind")).toBe("flag");
    expect(dialog()!.querySelector("[data-flag-field]")?.textContent).toContain("Keins");
    expect(button("Geschichte anlegen")?.disabled).toBe(true);
    // Nothing was sent; „Abbrechen“ closes.
    await click(button("Abbrechen"));
    expect(onCreate).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("creating_sends_name_progress_and_notes", async () => {
    await show();
    typeInto(field("Name"), "  Rivalin  ");
    // „Freie Variable nehmen“ in the picker takes the pool's smallest free variable.
    await click(dialog()!.querySelector<HTMLButtonElement>('[aria-label="Fortschritt wählen…"]'));
    await flush();
    await click([...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Freie Variable nehmen"));
    await flush();
    expect(freeFlag).toHaveBeenCalledWith("var");
    typeInto(field("Notizen"), "Die Rivalin begleitet den Weg.");
    // The create takes seconds: the button is locked while it runs, a second click sends nothing.
    const pending = deferred<void>();
    onCreate.mockImplementationOnce(() => pending.promise);
    await click(button("Geschichte anlegen"));
    expect(onCreate).toHaveBeenCalledWith({ via: "recipe", input: { name: "Rivalin", var: 0x406c, notes: "Die Rivalin begleitet den Weg." } });
    expect(button("Geschichte anlegen")?.disabled).toBe(true);
    expect(button("Abbrechen")?.disabled).toBe(true);
    expect(dialog()?.textContent).toContain("Wird angelegt …");
    await click(button("Geschichte anlegen"));
    expect(onCreate).toHaveBeenCalledOnce();
    expect(onClose).not.toHaveBeenCalled();
    await act(async () => pending.resolve());
    await flush();
    expect(onClose).toHaveBeenCalledOnce();
    // A flag without a name the game knows goes by its fallback id; empty notes stay out.
    onClose.mockReset();
    await show(false);
    await show();
    typeInto(field("Name"), "Medaille");
    await click(kindButton("Flag"));
    await pick("0x0173");
    await click(button("Geschichte anlegen"));
    await flush();
    expect(onCreate).toHaveBeenLastCalledWith({ via: "story", req: { name: "Medaille", progress: { flag: "FLAG_0371" } } });
  });

  it("a_backend_error_stays_in_the_dialog", async () => {
    await show();
    typeInto(field("Name"), "Rivalin");
    await pick("0x4040");
    onCreate.mockRejectedValueOnce({ code: "not_text_project", message: "Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.", asset: null });
    await click(button("Geschichte anlegen"));
    await flush();
    // The picked number goes to the recipe; the refusal stays in the dialog, which stays open and can try again.
    expect(onCreate).toHaveBeenCalledWith({ via: "recipe", input: { name: "Rivalin", var: 0x4040 } });
    expect(dialog()).not.toBeNull();
    expect(dialog()!.querySelector("[role=alert], [data-notice]")?.textContent ?? dialog()!.textContent).toContain("Kein Textprojekt: IDs gibt es erst nach");
    expect(onClose).not.toHaveBeenCalled();
    expect(button("Geschichte anlegen")?.disabled).toBe(false);
    // A new try that goes in takes the error away and closes.
    await click(button("Geschichte anlegen"));
    await flush();
    expect(onCreate).toHaveBeenCalledTimes(2);
    expect(onClose).toHaveBeenCalledOnce();
  });
});
