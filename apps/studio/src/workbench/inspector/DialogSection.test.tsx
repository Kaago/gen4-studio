// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dialogOf } from "../../api";
import type { Dialog, DialogSource } from "../../api";
import type { DialogTarget } from "../dialog/dialogModel";
import { DialogSection } from "./DialogSection";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), dialogOf: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const figure: DialogTarget = { header: 3, eventFile: 8, kind: "object", index: 2, title: "Objekt 2" };
const sign: DialogTarget = { header: 3, eventFile: 8, kind: "bg", index: 0, title: "Schild 0" };
const local: DialogSource = { kind: "local", file: 12, slot: 4, line: 7 };
const dialog = (source: DialogSource, texts: string[] = []): Dialog => ({
  source,
  messages: texts.map((text, i) => ({ archive: 554, index: 10 + i, text, command: "Message", line: 8 + i, col: 13, branch: null, shared_with: 0, split_refusal: null })),
  followed: [],
});

describe("DialogSection", () => {
  let container: HTMLElement;
  let root: Root;
  const onEdit = vi.fn();
  const onError = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(dialogOf).mockReset();
    onEdit.mockReset();
    onError.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  // Each answer under a new revision: the section reads the same figure again.
  let revision = 0;
  async function show(target: DialogTarget, answer: Dialog) {
    vi.mocked(dialogOf).mockResolvedValue(answer);
    revision++;
    await act(async () => root.render(<DialogSection target={target} revision={revision} onEdit={onEdit} onError={onError} />));
    await flush();
  }
  const section = () => container.querySelector<HTMLElement>("section[data-dialog-section]");
  const screen = () => container.querySelector<HTMLElement>('[role="group"][aria-label^="DS-Bildschirm"]');
  const button = (text: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === text);

  it("shows_the_first_page_summary_and_the_edit_button", async () => {
    await show(figure, dialog(local, [String.raw`Schönes Wetter!\rFindest du nicht?`, "Bis bald."]));
    expect(dialogOf).toHaveBeenCalledWith(3, 8, "object", 2);
    expect(section()?.querySelector("h3")?.textContent).toBe("Dialog");
    // The first page of the first message, in the text box.
    expect(screen()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 1 von 2");
    expect(screen()?.textContent).toContain("Schönes Wetter!");
    expect(screen()?.textContent).not.toContain("Findest du nicht?");
    expect(section()?.querySelector("[data-dialog-preview]")?.getAttribute("data-dialog-preview")).toBe("textbox");
    expect(section()?.textContent).toContain("2 Nachrichten · 3 Seiten");
    await act(async () => button("Dialog bearbeiten…")!.click());
    expect(onEdit).toHaveBeenCalledWith(figure);
  });

  it("a_sign_uses_the_sign_variant_and_the_title_text", async () => {
    await show(sign, dialog(local, ["Zweiblattdorf"]));
    expect(dialogOf).toHaveBeenCalledWith(3, 8, "bg", 0);
    expect(section()?.querySelector("h3")?.textContent).toBe("Text");
    expect(section()?.querySelector("[data-dialog-preview]")?.getAttribute("data-dialog-preview")).toBe("sign");
    expect(screen()?.textContent).toContain("Zweiblattdorf");
    expect(button("Dialog bearbeiten…")).not.toBeUndefined();
  });

  it("common_and_trainer_sources_show_the_sentence_without_a_button", async () => {
    await show(figure, dialog({ kind: "common", script: 2004 }));
    expect(section()?.textContent).toContain("Gemeinsames Script 2004: in Bauen bearbeiten.");
    expect(container.querySelectorAll("button")).toHaveLength(0);
    await show(figure, dialog({ kind: "trainer", id: 7 }));
    expect(section()?.textContent).toContain("Kampftexte des Trainers: im Bereich Pokémon.");
    expect(container.querySelectorAll("button")).toHaveLength(0);
    await show(figure, dialog({ kind: "unreadable", reason: "Erst Scripts übernehmen." }));
    expect(section()?.textContent).toContain("Script nicht lesbar: Erst Scripts übernehmen.");
    expect(container.querySelectorAll("button")).toHaveLength(0);
  });

  it("a_figure_without_a_script_opens_the_talk_recipe_from_the_dialog_section", async () => {
    const onStoryRecipe = vi.fn();
    vi.mocked(dialogOf).mockResolvedValue(dialog({ kind: "none" }));
    await act(async () =>
      root.render(<DialogSection target={figure} revision={++revision} onEdit={onEdit} onError={onError} figure={{ header: 3, local_id: 1 }} onStoryRecipe={onStoryRecipe} />),
    );
    await flush();
    expect(section()?.textContent).toContain("Sagt nichts");
    expect(button("Dialog bearbeiten…")).toBeUndefined();
    // „Dialog anlegen…“ is „Gespräch schaltet weiter“ with this figure: its story is chosen in the dialog.
    const anlegen = button("Dialog anlegen…")!;
    expect(anlegen.disabled).toBe(false);
    await act(async () => anlegen.click());
    expect(onStoryRecipe).toHaveBeenCalledExactlyOnceWith({ recipe: "talk_advances", prefill: { figure: { header: 3, local_id: 1 } } });
    // A sign has no recipe: no button.
    vi.mocked(dialogOf).mockResolvedValue(dialog({ kind: "none" }));
    await act(async () => root.render(<DialogSection target={sign} revision={++revision} onEdit={onEdit} onError={onError} />));
    await flush();
    expect(button("Dialog anlegen…")).toBeUndefined();
  });
});
