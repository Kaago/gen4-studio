// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { checkProjectDest, defaultProjectParent, inspectRom } from "../api";
import type { Created, RomInfo } from "../api";
import { ProjectStart } from "./ProjectStart";
import type { StartWizardRequest } from "./ProjectStart";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), inspectRom: vi.fn(), defaultProjectParent: vi.fn(), checkProjectDest: vi.fn(), createProject: vi.fn() }));
vi.mock("../platform/transport", () => ({ invoke: vi.fn(), inTauri: () => false }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

const PLATIN: RomInfo = {
  title: "POKEMON PL",
  game_code: "CPUD",
  family: "pt",
  language: "german",
  revision: 0,
  size: 134_217_728,
  sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4",
  known_base: "Platin DE (Testprojekt)",
  support: { kind: "verified" },
};

/** A DataTransfer as a browser hands it to `drop` (jsdom has none): the given types with their data, and files by name. */
function transfer(data: Record<string, string>, files: string[] = []): DataTransfer {
  const types = [...Object.keys(data), ...(files.length > 0 ? ["Files"] : [])];
  return { types, getData: (type: string) => data[type] ?? "", files: files.map((name) => ({ name })), dropEffect: "none" } as unknown as DataTransfer;
}

describe("ProjectStart", () => {
  let container: HTMLElement;
  let root: Root;
  const requests: (StartWizardRequest | null)[] = [];
  const onOpenCreated = vi.fn<(c: Created) => void>();
  const onNotice = vi.fn<(text: string) => void>();

  /** The start with its wizard request as state, the way App keeps it. */
  function Harness({ initial = null }: { initial?: StartWizardRequest | null }) {
    const [wizard, setWizard] = useState<StartWizardRequest | null>(initial);
    return (
      <ProjectStart
        recent={[]}
        missing={new Set()}
        onOpen={() => {}}
        onOpenRecent={() => {}}
        onForget={() => {}}
        previews={new Map()}
        onChoosePreview={() => {}}
        onRemovePreview={() => {}}
        wizard={wizard}
        onWizard={(r) => {
          requests.push(r);
          setWizard(r);
        }}
        onOpenCreated={onOpenCreated}
        onNotice={onNotice}
      />
    );
  }

  beforeEach(() => {
    requests.length = 0;
    onOpenCreated.mockReset();
    onNotice.mockReset();
    vi.mocked(inspectRom).mockReset().mockResolvedValue(PLATIN);
    vi.mocked(defaultProjectParent).mockReset().mockResolvedValue({ parent: "/roms/_testkopien", dspre_suffix: false });
    vi.mocked(checkProjectDest).mockReset().mockResolvedValue({ state: "missing", free_bytes: 50_000_000_000, needed_bytes: 335_544_320, ok: true });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(initial: StartWizardRequest | null = null) {
    await act(async () => root.render(<Harness initial={initial} />));
    await flush();
  }
  const surface = () => container.querySelector<HTMLElement>("[data-drop]")!;
  const dialog = () => container.querySelector<HTMLElement>('[role="dialog"][data-wizard]');
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
  const alerts = () => [...container.querySelectorAll("[role=alert]")].map((e) => e.textContent);
  /** A drag event as the browser delivers it, with its DataTransfer. */
  function drag(type: "dragenter" | "dragover" | "dragleave" | "drop", dt: DataTransfer, target: Element = surface()) {
    const event = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: dt });
    act(() => void target.dispatchEvent(event));
    return event;
  }

  it("shows_the_start_with_the_new_game_as_its_primary_action_and_opens_the_wizard", async () => {
    await show();
    expect(surface().dataset.drop).toBe("idle");
    expect(container.textContent).toContain("Willkommen zurück");
    expect(dialog()).toBeNull();
    const newGame = [...container.querySelectorAll("button")].find((b) => b.textContent === "Neues Spiel aus ROM…")!;
    await act(async () => newGame.click());
    await flush();
    expect(requests).toEqual([{}]);
    expect(dialog()).not.toBeNull();
    expect(current()).toBe("1. ROM");
  });

  it("shows_the_drop_frame_while_a_file_is_dragged_over", async () => {
    await show();
    const dt = transfer({}, ["pt.nds"]);
    // A drop target cancels both dragenter and dragover.
    const enter = drag("dragenter", dt);
    const over = drag("dragover", dt);
    expect(enter.defaultPrevented).toBe(true);
    expect(over.defaultPrevented).toBe(true);
    expect(surface().dataset.drop).toBe("over");
    expect(container.textContent).toContain("ROM hier ablegen");
    drag("dragleave", dt);
    expect(surface().dataset.drop).toBe("idle");
    expect(container.textContent).not.toContain("ROM hier ablegen");
  });

  it("a_drag_of_plain_text_can_be_dropped_but_shows_no_frame", async () => {
    await show();
    const dt = transfer({ "text/plain": "/roms/_testkopien/pt.nds" });
    const enter = drag("dragenter", dt);
    const over = drag("dragover", dt);
    expect(enter.defaultPrevented).toBe(true);
    expect(over.defaultPrevented).toBe(true);
    expect(surface().dataset.drop).toBe("idle");
    expect(container.textContent).not.toContain("ROM hier ablegen");
    // A uri list is a file from a file manager: it lights the frame.
    drag("dragenter", transfer({ "text/uri-list": "file:///roms/pt.nds" }));
    expect(surface().dataset.drop).toBe("over");
  });

  it("drop_of_a_rom_path_opens_the_wizard_with_it", async () => {
    await show();
    const dt = transfer({ "text/plain": "/roms/_testkopien/pt.nds" });
    drag("dragenter", dt);
    const dropped = drag("drop", dt);
    await flush();
    expect(dropped.defaultPrevented).toBe(true);
    expect(requests).toEqual([{ rom: "/roms/_testkopien/pt.nds" }]);
    expect(inspectRom).toHaveBeenCalledWith("/roms/_testkopien/pt.nds");
    // A released ROM moves on to the template.
    expect(current()).toBe("2. Vorlage");
  });

  it("drop_of_a_file_without_path_opens_the_wizard_with_the_notice", async () => {
    await show();
    drag("drop", transfer({}, ["pt.nds"]));
    await flush();
    expect(requests).toEqual([{ notice: "Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein." }]);
    expect(current()).toBe("1. ROM");
    expect(container.textContent).toContain("Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein.");
    expect(inspectRom).not.toHaveBeenCalled();
  });

  it("drop_of_two_files_or_a_txt_is_refused", async () => {
    await show();
    drag("drop", transfer({}, ["a.nds", "b.nds"]));
    await flush();
    expect(alerts()).toContain("Nur eine .nds-Datei ablegen.");
    drag("drop", transfer({ "text/plain": "/roms/notes.txt" }));
    drag("drop", transfer({}, ["notes.txt"]));
    await flush();
    expect(requests).toEqual([]);
    expect(dialog()).toBeNull();
    expect(surface().dataset.drop).toBe("idle");
  });

  it("drop_while_the_wizard_is_open_is_refused_with_a_hint_and_keeps_the_wizard", async () => {
    await show({});
    expect(dialog()).not.toBeNull();
    const dt = transfer({ "text/plain": "/roms/_testkopien/pt.nds" }, ["pt.nds"]);
    const over = drag("dragover", dt, dialog()!);
    const dropped = drag("drop", dt, dialog()!);
    await flush();
    // The browser must not open the file in place of the app, and the wizard keeps what it has.
    expect(over.defaultPrevented).toBe(true);
    expect(dropped.defaultPrevented).toBe(true);
    expect(requests).toEqual([]);
    expect(inspectRom).not.toHaveBeenCalled();
    expect(current()).toBe("1. ROM");
    expect(onNotice).toHaveBeenCalledExactlyOnceWith("Erst den Assistenten abschließen oder abbrechen.");
    // A file path dragged as a uri list is refused too; a plain text drag (into a field) is left alone.
    drag("drop", transfer({ "text/uri-list": "file:///roms/pt.nds" }), dialog()!);
    expect(onNotice).toHaveBeenCalledTimes(2);
    drag("drop", transfer({ "text/plain": "/roms/pt.nds" }), dialog()!);
    expect(onNotice).toHaveBeenCalledTimes(2);
    expect(requests).toEqual([]);
  });

  it("wizard_renders_into_its_own_hinge_and_hand", async () => {
    await show({});
    const bar = container.querySelector<HTMLElement>("[data-wizard-bar]");
    const hand = container.querySelector<HTMLElement>("[data-wizard-hand]");
    expect(bar).not.toBeNull();
    expect(hand).not.toBeNull();
    expect(bar?.hidden).toBe(false);
    expect(bar?.querySelector('ol[aria-label="Schritte"]')).not.toBeNull();
    expect(hand?.querySelector('[role="dialog"][data-wizard]')).not.toBeNull();
    // The view shows what is chosen so far: no ROM yet.
    const schau = container.querySelector<HTMLElement>("[data-start-schau]")!;
    expect(schau.textContent).toContain("Noch keine ROM");
    // Cancel closes the wizard and brings the start back.
    const cancel = [...container.querySelectorAll("button")].find((b) => b.textContent === "Abbrechen")!;
    await act(async () => cancel.click());
    expect(requests).toEqual([null]);
    expect(dialog()).toBeNull();
    expect(container.textContent).toContain("Willkommen zurück");
    // The focus comes back to the main action.
    expect(document.activeElement?.textContent).toBe("Neues Spiel aus ROM…");
  });

  it("the_view_follows_the_choice_of_the_wizard", async () => {
    await show({ rom: "/roms/_testkopien/pt.nds" });
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 260)));
    const schau = container.querySelector<HTMLElement>("[data-start-schau]")!;
    expect(schau.querySelector('[aria-label^="DS-Bildschirm"]')?.textContent).toContain("POKEMON PL");
    expect(schau.textContent).toContain("Platin (Deutsch)");
    expect(schau.textContent).toContain("Original");
    expect(schau.textContent).toContain("/roms/_testkopien/Neues Platin");
  });
});
