// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary, TextArchiveDto } from "../../api";
import { createTextArchive, editTextArchive, getTextArchive, searchText, validateText } from "../../api";
import { TextPanel } from "./TextPanel";
import type { TextPanelContext } from "./TextPanel";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  getTextArchive: vi.fn(),
  editTextArchive: vi.fn(),
  createTextArchive: vi.fn(),
  searchText: vi.fn(),
  validateText: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const header = (id: number, text_archive_id: number): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name: "",
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id,
});

const archive = (index: number, texts: string[]): TextArchiveDto => ({
  index,
  key: 1,
  lang: "de",
  messages: texts.map((text, i) => ({ id: `msg_${String(i).padStart(5, "0")}`, text })),
  users: [],
});

const TWINLEAF = archive(433, ["Nowhere", "Twinleaf Town", "Sandgem Town"]);

describe("TextPanel", () => {
  let container: HTMLElement;
  let root: Root;
  const onEdited = vi.fn();
  const onCountsChanged = vi.fn();
  const onError = vi.fn();
  const goTo = vi.fn();
  const base = (over: Partial<TextPanelContext> = {}): TextPanelContext => ({
    headers: [header(12, 433), header(13, 433), header(14, 5)],
    counts: { TextArchive: 435 },
    target: null,
    revision: 0,
    goTo,
    onEdited,
    onCountsChanged,
    onError,
    ...over,
  });
  const show = (context: TextPanelContext) => act(() => root.render(<TextPanel context={context} />));

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    vi.mocked(getTextArchive).mockReset().mockImplementation(async (index) => (index === 433 ? TWINLEAF : archive(index, ["Neuer Text"])));
    vi.mocked(editTextArchive).mockReset().mockResolvedValue(1);
    vi.mocked(createTextArchive).mockReset().mockResolvedValue(435);
    vi.mocked(searchText).mockReset().mockResolvedValue([{ archive: 433, index: 1, excerpt: "…Twinleaf Town…" }]);
    vi.mocked(validateText).mockReset().mockResolvedValue([]);
    for (const f of [onEdited, onCountsChanged, onError, goTo]) f.mockReset();
    // jsdom has no layout, hence no scrollIntoView.
    Element.prototype.scrollIntoView = () => {};
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.useRealTimers();
  });

  const archiveButton = (index: number) => container.querySelector<HTMLButtonElement>(`[data-archive="${index}"]`);
  const messageButton = (index: number) => container.querySelector<HTMLButtonElement>(`[data-message="${index}"]`);
  const textarea = () => container.querySelector<HTMLTextAreaElement>("textarea");
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => b.textContent === name);
  const searchBox = () => container.querySelector<HTMLInputElement>('input[type="search"]');
  /** Types into a controlled React field the way a user does. */
  function type(field: HTMLInputElement | HTMLTextAreaElement, value: string) {
    const proto = field instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    act(() => {
      Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(field, value);
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
  }
  const blur = (field: HTMLElement) => act(() => void field.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());

  async function open433() {
    show(base());
    await flush();
    click(archiveButton(433));
    await flush();
  }

  it("lists every archive with its number and the headers that use it, without reading any archive", async () => {
    show(base());
    await flush();
    expect(container.querySelectorAll("[data-archive]")).toHaveLength(435);
    expect(archiveButton(433)?.textContent).toBe("Textarchiv 0433 · Header 0012, 0013");
    expect(archiveButton(5)?.textContent).toBe("Textarchiv 0005 · Header 0014");
    expect(archiveButton(6)?.textContent).toBe("Textarchiv 0006");
    expect(getTextArchive).not.toHaveBeenCalled();
  });

  it("loads the messages of a clicked archive and shows them with visible control characters", async () => {
    vi.mocked(getTextArchive).mockResolvedValueOnce(archive(433, ["Zeile\\nzwei\\rdrei\\f", "Eins"]));
    await open433();
    expect(getTextArchive).toHaveBeenCalledWith(433);
    expect(messageButton(0)?.textContent).toContain("Zeile⏎zwei▼drei▽");
    expect(messageButton(1)?.textContent).toContain("Eins");
    // The first message is selected; the field shows the text as stored.
    expect(textarea()?.value).toBe("Zeile\\nzwei\\rdrei\\f");
    // The count of the opened archive is known now.
    expect(container.textContent).toContain("2 Nachrichten");
  });

  it("shows a decode error in the message area and keeps the list working", async () => {
    vi.mocked(getTextArchive).mockImplementation(async (index) => {
      if (index === 7) throw { code: "error", message: "Textarchiv 0007 nicht lesbar", asset: null };
      return TWINLEAF;
    });
    show(base());
    await flush();
    click(archiveButton(7));
    await flush();
    expect(container.textContent).toContain("Textarchiv 0007 nicht lesbar");
    expect(textarea()).toBeNull();
    expect(onError).not.toHaveBeenCalled();
    click(archiveButton(433));
    await flush();
    expect(messageButton(1)?.textContent).toContain("Twinleaf Town");
    expect(container.textContent).not.toContain("nicht lesbar");
  });

  it("commits a typed text on blur as one Set edit and refreshes the dirty count", async () => {
    await open433();
    click(messageButton(1));
    expect(textarea()?.value).toBe("Twinleaf Town");
    type(textarea()!, "Zweiblattdorf");
    expect(editTextArchive).not.toHaveBeenCalled();
    blur(textarea()!);
    await flush();
    expect(editTextArchive).toHaveBeenCalledTimes(1);
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Zweiblattdorf" } });
    expect(onEdited).toHaveBeenCalled();
    // A blur without a change writes nothing.
    blur(textarea()!);
    await flush();
    expect(editTextArchive).toHaveBeenCalledTimes(1);
  });

  it("stores a typed or pasted line break as the escape the archive uses", async () => {
    await open433();
    click(messageButton(1));
    type(textarea()!, "Zwei\nZeilen\r\nDrei");
    expect(textarea()?.value).toBe("Zwei\\nZeilen\\nDrei");
    blur(textarea()!);
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Zwei\\nZeilen\\nDrei" } });
  });

  it("colours the control codes of the selected message in a preview below the field", async () => {
    vi.mocked(getTextArchive).mockResolvedValueOnce(archive(433, ["Hi {PLAYER}\\n[PK]"]));
    await open433();
    const kinds = [...container.querySelectorAll("[data-segment]")].map((e) => `${e.getAttribute("data-segment")}:${e.textContent}`);
    expect(kinds).toEqual(["text:Hi ", "code:{PLAYER}", "newline:⏎", "code:[PK]"]);
  });

  it("commits after 500 ms of rest, once", async () => {
    await open433();
    click(messageButton(1));
    vi.useFakeTimers();
    type(textarea()!, "Zweiblatt");
    type(textarea()!, "Zweiblattdorf");
    await act(() => vi.advanceTimersByTimeAsync(400));
    expect(editTextArchive).not.toHaveBeenCalled();
    await act(() => vi.advanceTimersByTimeAsync(200));
    expect(editTextArchive).toHaveBeenCalledTimes(1);
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Zweiblattdorf" } });
    await act(() => vi.advanceTimersByTimeAsync(2000));
    expect(editTextArchive).toHaveBeenCalledTimes(1);
  });

  it("commits a pending text before it switches to another message", async () => {
    await open433();
    click(messageButton(1));
    type(textarea()!, "Neu");
    click(messageButton(2));
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(433, { Set: { index: 1, text: "Neu" } });
    expect(textarea()?.value).toBe("Sandgem Town");
  });

  it("reports a refused edit like any command error", async () => {
    const refused = { code: "error", message: "text archive 433 has 3 messages, none at 3", asset: null };
    vi.mocked(editTextArchive).mockRejectedValueOnce(refused);
    await open433();
    type(textarea()!, "x");
    blur(textarea()!);
    await flush();
    expect(onError).toHaveBeenCalledWith(refused);
  });

  it("warns about a text that does not fit, from validateText", async () => {
    vi.mocked(validateText).mockResolvedValue([{ severity: "warning", file: "", line: 2, col: 1, message: "line is 260 px wide, the box holds 224 px", source: "text" }]);
    await open433();
    vi.useFakeTimers();
    type(textarea()!, "W".repeat(60));
    await act(() => vi.advanceTimersByTimeAsync(350));
    expect(validateText).toHaveBeenLastCalledWith("W".repeat(60));
    expect(container.textContent).toContain("Zeile 2: line is 260 px wide, the box holds 224 px");
  });

  it("appends a message and selects it, and removes the last one", async () => {
    await open433();
    vi.mocked(getTextArchive).mockResolvedValue(archive(433, ["Nowhere", "Twinleaf Town", "Sandgem Town", ""]));
    click(button("Nachricht anfügen"));
    await flush();
    expect(editTextArchive).toHaveBeenCalledWith(433, "Append");
    expect(messageButton(3)).not.toBeNull();
    expect(messageButton(3)?.getAttribute("aria-current")).toBe("true");

    vi.mocked(getTextArchive).mockResolvedValue(TWINLEAF);
    click(button("Letzte entfernen"));
    await flush();
    expect(editTextArchive).toHaveBeenLastCalledWith(433, "RemoveLast");
    expect(messageButton(3)).toBeNull();
    // The selection moved to a message that exists.
    expect(messageButton(2)?.getAttribute("aria-current")).toBe("true");
  });

  it("searches after 300 ms and selects the archive and message of a clicked hit", async () => {
    show(base());
    await flush();
    vi.useFakeTimers();
    type(searchBox()!, "twin");
    await act(() => vi.advanceTimersByTimeAsync(250));
    expect(searchText).not.toHaveBeenCalled();
    await act(() => vi.advanceTimersByTimeAsync(100));
    expect(searchText).toHaveBeenCalledTimes(1);
    expect(vi.mocked(searchText).mock.calls[0][0]).toBe("twin");
    vi.useRealTimers();
    await flush();
    const hit = container.querySelector<HTMLButtonElement>('[data-hit="433:1"]');
    expect(hit?.textContent).toBe("0433 · 1 · …Twinleaf Town…");
    click(hit);
    await flush();
    expect(getTextArchive).toHaveBeenCalledWith(433);
    expect(messageButton(1)?.getAttribute("aria-current")).toBe("true");
    expect(textarea()?.value).toBe("Twinleaf Town");
  });

  it("filters the archive list by number without asking the backend", async () => {
    show(base());
    await flush();
    type(searchBox()!, "0433");
    expect([...container.querySelectorAll("[data-archive]")].map((e) => e.getAttribute("data-archive"))).toEqual(["433"]);
  });

  it("creates a new archive, refreshes the counts and selects it", async () => {
    show(base());
    await flush();
    click(button("Neues Archiv"));
    await flush();
    expect(createTextArchive).toHaveBeenCalledTimes(1);
    expect(onCountsChanged).toHaveBeenCalled();
    expect(getTextArchive).toHaveBeenCalledWith(435);
    expect(textarea()?.value).toBe("Neuer Text");
  });

  it("opens the archive and message a link led to", async () => {
    show(base({ target: { index: 433, message: 2 } }));
    await flush();
    expect(getTextArchive).toHaveBeenCalledWith(433);
    expect(messageButton(2)?.getAttribute("aria-current")).toBe("true");
    expect(textarea()?.value).toBe("Sandgem Town");
  });

  it("reads the open archive again when undo or redo moved it (revision), and not for the same context", async () => {
    await open433();
    const calls = vi.mocked(getTextArchive).mock.calls.length;
    show(base());
    await flush();
    expect(vi.mocked(getTextArchive).mock.calls.length).toBe(calls);
    vi.mocked(getTextArchive).mockResolvedValue(archive(433, ["Nowhere", "Zurück", "Sandgem Town"]));
    show(base({ revision: 1 }));
    await flush();
    expect(messageButton(1)?.textContent).toContain("Zurück");
    expect(textarea()?.value).toBe("Nowhere");
  });

  it("links the headers of the open archive", async () => {
    await open433();
    click(container.querySelector('[data-header="12"]'));
    expect(goTo).toHaveBeenCalledWith({ kind: "header", id: 12 });
  });
});
