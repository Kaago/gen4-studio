// Shared by the tests of the text area: sample data, the area mounted with all its parts, and the ways a user types and clicks.
// Each test file mocks `../../api` (and the thumbnail store) itself: `vi.mock` is hoisted per file.
import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { vi } from "vitest";
import type { HeaderSummary, ProjectInfo, TextArchiveDto } from "../../api";
import { createEditBus } from "../../bus/editBus";
import type { EditEvent } from "../../bus/editBus";
import { ProblemsStore } from "../problems/problems";
import { TextArea, TextHints, TextStatus } from "./TextArea";
import type { TextAreaProps } from "./TextArea";
import { TextHand } from "./TextHand";
import { TextList } from "./TextList";
import { TextView } from "./TextView";

export const header = (id: number, text_archive_id: number, location_name = ""): HeaderSummary => ({
  id,
  internal_name: `H${id}`,
  location_name,
  matrix_id: 0,
  area_data_id: 0,
  event_file_id: 0,
  text_archive_id,
  script_file_id: 0,
  level_script_id: 0,
});

export const archive = (index: number, texts: string[]): TextArchiveDto => ({
  index,
  key: 1,
  lang: "de",
  messages: texts.map((text, i) => ({ id: `msg_${String(i).padStart(5, "0")}`, text })),
  users: [],
});

export const projectA: ProjectInfo = { root: "/a", game_code: "CPUE", family: "Pt", language: "en", header_count: 3 };

export const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

/** The area with its list, view, hand and status parts, in a fresh container; `base` builds a context with mock callbacks. */
export function textHarness() {
  const container = document.createElement("div");
  document.body.append(container);
  const root: Root = createRoot(container);
  const problems = new ProblemsStore();
  const calls = { onEdited: vi.fn(), onCountsChanged: vi.fn(), onError: vi.fn(), goTo: vi.fn(), onShown: vi.fn() };
  // What the area tells the rest of the app.
  const bus = createEditBus();
  const events: EditEvent[] = [];
  bus.subscribe(() => true, (event) => void events.push(event));
  const base = (over: Partial<TextAreaProps> = {}): TextAreaProps => ({
    bus,
    project: projectA,
    headers: [header(12, 433, "Zweiblattdorf"), header(13, 433, "Zweiblattdorf"), header(14, 5, "Sandgemme")],
    counts: { TextArchive: 435 },
    target: null,
    revision: 0,
    problems,
    ...calls,
    ...over,
  });
  const tree = (context: TextAreaProps) => (
    <TextArea context={context}>
      <TextList open onToggle={() => {}} />
      <TextView />
      <TextHand />
      <p data-status="">
        <TextStatus />
      </p>
      <p data-hints="">
        <TextHints />
      </p>
    </TextArea>
  );
  const show = (context: TextAreaProps) => act(() => root.render(tree(context)));
  const showStrict = (context: TextAreaProps) => act(() => root.render(<StrictMode>{tree(context)}</StrictMode>));

  const q = <E extends Element = HTMLElement>(selector: string) => container.querySelector<E>(selector);
  const archiveRow = (index: number) => q(`[data-archive="${index}"]`);
  const messageRow = (index: number) => q(`[data-message="${index}"]`);
  const textarea = () => q<HTMLTextAreaElement>('textarea[aria-label="Text der Nachricht"]');
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const searchBox = () => q<HTMLInputElement>('input[aria-label="Texte suchen"]');
  const listFilter = () => q<HTMLInputElement>('input[aria-label="Textarchive filtern"]');
  const screen = () => q('[role="group"][aria-label^="DS-Bildschirm"]');

  /** Types into a controlled React field the way a user does. */
  function type(field: HTMLInputElement | HTMLTextAreaElement, value: string) {
    const proto = field instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    act(() => {
      Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(field, value);
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
  }
  const blur = (field: HTMLElement) => act(() => void field.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
  const focus = (field: HTMLElement) => act(() => field.focus());
  const click = (el: Element | null | undefined) => act(() => void (el as HTMLElement).click());
  const key = (el: Element, init: KeyboardEventInit) => act(() => void el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init })));

  async function open433() {
    show(base());
    await flush();
    click(archiveRow(433));
    await flush();
  }

  function unmount() {
    act(() => root.unmount());
    container.remove();
  }

  return { container, root, problems, calls, bus, events, base, show, showStrict, archiveRow, messageRow, textarea, button, searchBox, listFilter, screen, type, blur, focus, click, key, open433, unmount };
}
