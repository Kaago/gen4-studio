// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dialogOf, dialogsOfHeader, getTextArchive } from "../../api";
import { TRAINER_ROWS } from "../../data/trainer/trainerTesting";
import { figure, flush, headerDialogs, textHarness } from "./textTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getTextArchive: vi.fn(),
  dialogsOfHeader: vi.fn(),
  dialogOf: vi.fn(),
  validateText: vi.fn(async () => []),
}));
vi.mock("../../world/thumbnailStore", () => ({
  thumbnails: { get: () => undefined, request: () => {}, cancel: () => {}, subscribe: () => () => {}, failure: () => null },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const FIGURES = [
  figure("object", 3, { first_text: String.raw`Schönes Wetter heute!\rNicht wahr?`, messages: 2, pages: 3, message_ids: [4, 5] }),
  figure("object", 4, { source: { kind: "none" }, first_text: null, messages: 0, pages: 0, message_ids: [] }),
  figure("object", 5, { source: { kind: "trainer", id: 12 }, trainer: 12, first_text: null, messages: 0, pages: 0, message_ids: [] }),
  figure("object", 6, { source: { kind: "common", script: 2001 }, first_text: "Willkommen!", messages: 1, pages: 1, message_ids: [] }),
  figure("bg", 0, { first_text: "Zweiblattdorf", messages: 1, pages: 1, message_ids: [0] }),
];

describe("DialogCards", () => {
  let h: ReturnType<typeof textHarness>;
  const card = (key: string) => h.container.querySelector<HTMLElement>(`[data-dialog-figure="${key}"]`);
  const cards = () => h.container.querySelector<HTMLElement>("[data-dialog-cards]");

  async function openHeader12(over: Parameters<typeof h.base>[0] = {}) {
    h.show(h.base({ depth: "erzaehlen", ...over }));
    await flush();
    h.click(h.container.querySelector('[data-dialog-place="Zweiblattdorf"]'));
    await flush();
    h.click(h.container.querySelector('[data-dialog-header="12"]'));
    await flush();
  }

  beforeEach(() => {
    h = textHarness();
    vi.mocked(getTextArchive).mockReset();
    vi.mocked(dialogOf).mockReset().mockResolvedValue({ source: { kind: "none" }, messages: [], followed: [] });
    vi.mocked(dialogsOfHeader)
      .mockReset()
      .mockImplementation(async (id) => headerDialogs(id, id === 12 ? FIGURES : []));
    Element.prototype.scrollIntoView = () => {};
  });
  afterEach(() => h.unmount());

  it("asks for a place while none is chosen", async () => {
    h.show(h.base({ depth: "erzaehlen" }));
    await flush();
    expect(cards()).toBeNull();
    expect(h.container.textContent).toContain("Wähle links einen Ort.");
  });

  it("shows one card per figure with the first page on the DS screen and what it says", async () => {
    await openHeader12({ trainers: TRAINER_ROWS });
    expect([...cards()!.querySelectorAll("[data-dialog-figure]")].map((e) => e.getAttribute("data-dialog-figure"))).toEqual(["object:3", "object:4", "object:5", "object:6", "bg:0"]);
    const talker = card("object:3")!;
    expect(talker.textContent).toContain("Objekt 3");
    expect(talker.textContent).toContain("2 Nachrichten · 3 Seiten");
    const screen = talker.querySelector('[aria-label^="DS-Bildschirm"]');
    expect(screen?.textContent).toContain("Schönes Wetter heute!");
    expect(screen?.textContent).not.toContain("Nicht wahr?");
    // A figure without a script is muted.
    expect(card("object:4")?.textContent).toContain("Sagt nichts");
    expect(card("object:4")?.hasAttribute("data-muted")).toBe(true);
    // The trainer by class and name, and where its texts are.
    expect(card("object:5")?.textContent).toContain("Trainer Wanderer Jens");
    expect(card("object:5")?.textContent).toContain("Kampftexte des Trainers: im Bereich Pokémon");
    expect(card("object:6")?.textContent).toContain("Gemeinsames Script: in Bauen bearbeiten.");
    expect(card("object:6")?.textContent).not.toContain("2001");
    expect(card("bg:0")?.textContent).toContain("Schild 0");
    expect(card("bg:0")?.textContent).toContain("1 Nachricht · 1 Seite");
    // Only figures with a script of their own open the editor; the trainer opens the trainer.
    const buttons = (key: string) => [...card(key)!.querySelectorAll("button")].map((b) => b.textContent);
    expect(buttons("object:3")).toContain("Dialog bearbeiten…");
    expect(buttons("bg:0")).toContain("Dialog bearbeiten…");
    expect(buttons("object:4")).not.toContain("Dialog bearbeiten…");
    expect(buttons("object:6")).not.toContain("Dialog bearbeiten…");
    expect(buttons("object:5")).toEqual(expect.arrayContaining(["Trainer öffnen"]));
    expect(buttons("object:5")).not.toContain("Dialog bearbeiten…");
    // No archive numbers in Erzählen.
    expect(cards()!.textContent).not.toMatch(/Archiv|0433/);
  });

  it("reads the trainer names when a trainer stands there, and names it without a number until then", async () => {
    await openHeader12();
    expect(h.calls.needTrainers).toHaveBeenCalled();
    expect(card("object:5")?.querySelector("button")?.getAttribute("aria-label")).toBe("Trainer");
    expect(card("object:5")?.textContent).not.toMatch(/0012|\b12\b/);
  });

  it("one double click on a card's key opens once", async () => {
    await openHeader12({ trainers: TRAINER_ROWS });
    const doubleClick = (el: Element) =>
      act(() => {
        el.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 1 }));
        el.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 2 }));
        el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true, detail: 2 }));
      });
    const key = (card: string, name: string) => [...h.container.querySelectorAll(`[data-dialog-figure="${card}"] button`)].find((b) => b.textContent === name)!;
    doubleClick(key("object:3", "Dialog bearbeiten…"));
    expect(h.calls.onOpenDialog).toHaveBeenCalledTimes(1);
    doubleClick(key("object:5", "Trainer öffnen"));
    expect(h.calls.goTo).toHaveBeenCalledTimes(1);
    // A double click elsewhere on the card opens too, once.
    doubleClick(card("object:3")!.querySelector('[aria-label^="DS-Bildschirm"]')!);
    expect(h.calls.onOpenDialog).toHaveBeenCalledTimes(2);
  });
});
