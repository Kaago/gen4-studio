// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LANGUAGE_KEY } from "./i18n";
import { report } from "./checks/checksTesting";
import { noRepo } from "./git/gitTesting";
import { releasesOf } from "./publish/publishTesting";

/**
 * The language switch of Einstellungen over the real App: the panels are stubs, the deck renders its hinge, the transport answers the few
 * commands the start and the settings ask for. The world's list is a module-level element that reads its word through `termLabel` and
 * subscribes to nothing: only a remount shows it in the new language.
 */
/** What App hands the workbench and the data areas in the last render: the texts' target rides in `data`, the data areas' in `target`. */
const shown = vi.hoisted(() => ({ data: null as null | { textTarget: unknown; goTo: (target: unknown) => unknown }, dataTarget: undefined as unknown }));
vi.mock("./workbench/Workbench", () => ({
  default: ({ data, children }: { data: typeof shown.data; children: unknown }) => {
    shown.data = data;
    return children;
  },
}));
vi.mock("./workbench/panels", async () => {
  const { termLabel } = await import("./workbench/terms");
  return {
    ExplorerPane: () => <span data-probe="explorer">{termLabel("worldMap")}</span>,
    TextArchivesPane: () => null,
    PokemonListPane: () => null,
    ItemsListPane: () => null,
    TextAreaHost: ({ children }: { children: unknown }) => children,
    DataAreaHost: ({ target, children }: { target: unknown; children: unknown }) => {
      shown.dataTarget = target;
      return children;
    },
    TextHints: () => null,
    TextStatus: () => null,
    PokemonStatus: () => null,
    ItemsStatus: () => null,
    DECK_COMPONENTS: { welt: {}, texte: {}, pokemon: {}, gegenstaende: {} },
  };
});
vi.mock("./ui/frame/Deck", () => ({
  resetLayout: () => {},
  Deck: ({ area, hinge }: { area: string; hinge: unknown }) => (
    <>
      {hinge as never}
      <div data-dock={area} />
    </>
  ),
}));
/** `tauri`: whether the page pretends to run inside the Tauri window (the language test below does: the backend is there to be told). */
const env = vi.hoisted(() => ({ tauri: false }));
/** Every command the transport was given, in order, with the language i18next showed at that moment. */
const sent = vi.hoisted(() => [] as { cmd: string; shown: string }[]);
const answers: Record<string, unknown> = {
  agent_settings: { settings: { version: 1, enabled: false, expert: false, grants: [] }, setup_line: "claude mcp add gen4-studio -- \"/x\" mcp", open_project: null },
  open_project: { root: "/p", game_code: "CPUE", family: "HGSS", language: "en", header_count: 0 },
  list_headers: { headers: [], names_error: null },
  member_counts: {},
  dirty: [],
  history: { undo: [], redo: [] },
  list_matrices: { matrices: [], unreadable: [] },
  list_warps: { links: [], unreadable: [] },
  data_list: [],
  name_tables: { species: [], moves: [], items: [], abilities: [], types: [], classes: [], forms: [], archives: {}, sounds: [] },
  item_table: null,
  run_checks: report(),
  checks_status: report({ groups: [] }),
  git_status: noRepo(),
  text_project_status: { state: "dspre", adoptable: true, reason: null },
  base_status: { state: "unassigned", suggestion: null },
  releases: releasesOf(),
  original_status: { state: "unavailable", reason: "Diesem Projekt ist keine Basis-ROM zugeordnet." },
  world_meta_get: { meta: { version: 1, worldMaps: [], notWorldMaps: [], roomNames: {}, roomOwner: {} }, broken: null },
  system_language: null,
};
const invoke = vi.hoisted(() => vi.fn());
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => env.tauri }));
// Inside Tauri the dialog is the system's: the folder to open is given.
vi.mock("./platform/dialogs", async (importOriginal) => ({ ...(await importOriginal<typeof import("./platform/dialogs")>()), openPath: async () => "/p" }));
// Play mode stays off: its transport answers every command with undefined (useLive has its own tests).
vi.mock("./platform/liveEvents", async (importOriginal) => ({ ...(await importOriginal<typeof import("./platform/liveEvents")>()), subscribeLive: () => () => {} }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ onCloseRequested: async () => () => {} }) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));

import App from "./App";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
/** A switch waits for the backend (`set_locale`) before the app shows the new language: waits until it does, then lets the render settle. */
const switchedTo = async (lang: string) => {
  await act(async () => {
    await vi.waitFor(() => expect(i18next.language).toBe(lang));
  });
  await flush();
};

describe("App language switch", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    localStorage.clear();
    sent.length = 0;
    invoke.mockReset();
    invoke.mockImplementation(async (cmd: string) => {
      sent.push({ cmd, shown: i18next.language });
      return answers[cmd];
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    localStorage.clear();
    env.tauri = false;
    await i18next.changeLanguage("de");
  });

  const atlas = (area: string) => container.querySelector<HTMLButtonElement>(`nav [data-area="${area}"]`)!;
  const languageOption = (text: string) =>
    [...container.querySelectorAll<HTMLButtonElement>("[role=group] button")].find((b) => b.textContent === text && b.closest("[role=group]")?.textContent?.includes("Deutsch"));

  it("shows every part below the frame in the language picked in Einstellungen, and stays in Einstellungen", async () => {
    await act(async () => root.render(<App />));
    await flush();
    // The world's list is built on its first visit and stays (hidden) while another area shows.
    await act(async () => atlas("welt").click());
    expect(container.querySelector("[data-probe=explorer]")?.textContent).toBe("Weltkarte");
    await act(async () => atlas("einstellungen").click());
    await flush();

    await act(async () => languageOption("English")!.click());
    await switchedTo("en");

    expect(i18next.language).toBe("en");
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe("en");
    // A text read through `termLabel` in an element that neither subscribes nor gets new props: remounted, it reads English.
    expect(container.querySelector("[data-probe=explorer]")?.textContent).toBe("World map");
    // Still in the settings, which speak English now.
    expect(atlas("einstellungen").getAttribute("aria-current")).toBe("page");
    expect(container.querySelector("h1")?.textContent).toBe("Settings");
    expect(languageOption("English")?.getAttribute("aria-pressed")).toBe("true");
    // The remount took the focus from the option just pressed: it is given back.
    expect(document.activeElement).toBe(languageOption("English"));
  });

  it("forgets the last link targets on a switch, so the remounted areas do not follow them again", async () => {
    await act(async () => root.render(<App />));
    await flush();
    // A link to a species and one to a message, the way the panels follow them: the targets wait for the areas.
    await act(async () => void shown.data!.goTo({ kind: "species", index: 397 }));
    await act(async () => void shown.data!.goTo({ kind: "textArchive", index: 23, message: 2 }));
    expect(shown.dataTarget).toEqual({ sub: "pokedex", index: 397 });
    expect(shown.data!.textTarget).toEqual({ index: 23, message: 2 });
    await act(async () => atlas("einstellungen").click());
    await flush();

    await act(async () => languageOption("English")!.click());
    await switchedTo("en");

    // The areas mount anew with no target: they keep what the user chose since, instead of jumping back to the old link.
    expect(shown.dataTarget).toBeNull();
    expect(shown.data!.textTarget).toBeNull();
    expect(atlas("einstellungen").getAttribute("aria-current")).toBe("page");
  });

  it("leaves the focus alone the next time Einstellungen shows after a choice that keeps the language", async () => {
    await act(async () => root.render(<App />));
    await flush();
    await act(async () => atlas("einstellungen").click());
    await flush();
    // German is shown already: „Deutsch“ changes the choice, not the language, so nothing remounts.
    await act(async () => languageOption("Deutsch")!.click());
    await flush();
    expect(i18next.language).toBe("de");
    await act(async () => atlas("welt").click());
    const outside = document.createElement("button");
    document.body.append(outside);
    outside.focus();
    await act(async () => atlas("einstellungen").click());
    await flush();
    // A fresh visit to the settings does not pull the focus to the language control.
    expect(document.activeElement).toBe(outside);
    outside.remove();
  });

  describe("inside the app, with a project open", () => {
    const count = (cmd: string) => sent.filter((c) => c.cmd === cmd).length;
    const click = async (label: string) => {
      const button = [...container.querySelectorAll("button")].find((b) => b.textContent === label || b.getAttribute("aria-label") === label);
      if (!button) throw new Error(`no button ${label}`);
      await act(async () => button.click());
    };

    it("tells the backend the new language first, then reads again what it built in the old one", async () => {
      env.tauri = true;
      await act(async () => root.render(<App />));
      await flush();
      await click("Projekt öffnen");
      await flush();
      // The palette asks for every data list (the store reads what was asked for); closed again, the lists stay wanted.
      const paletteButton = [...container.querySelectorAll<HTMLButtonElement>("header button")].find((b) => b.textContent?.includes("Suchen oder Befehl eingeben"))!;
      await act(async () => paletteButton.click());
      await flush();
      const palette = document.querySelector<HTMLInputElement>("[role=dialog] input[role=combobox]")!;
      act(() => void palette.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })));
      await flush();
      // What the open project asked for so far, once or more: the overview (matrices, event files), the data lists and names the palette
      // asked for, the checks, the Git state, the project's state as a text project and the base ROM's state for resetting.
      const reads = ["list_matrices", "list_warps", "data_list", "name_tables", "run_checks", "git_status", "text_project_status", "original_status"] as const;
      const before = Object.fromEntries(reads.map((cmd) => [cmd, count(cmd)]));
      for (const cmd of reads) expect(before[cmd], cmd).toBeGreaterThan(0);
      const mark = sent.length;

      await act(async () => atlas("einstellungen").click());
      await flush();
      await act(async () => languageOption("English")!.click());
      await switchedTo("en");

      // The backend heard the language while the app still showed German ...
      const told = sent.findIndex((c, i) => i >= mark && c.cmd === "set_locale");
      expect(told).toBeGreaterThanOrEqual(mark);
      expect(sent[told]!.shown).toBe("de");
      expect(invoke).toHaveBeenCalledWith("set_locale", { lang: "en" });
      expect(i18next.language).toBe("en");
      // ... and each of the reads above was made again, after that, so what came back is in English.
      for (const cmd of reads) {
        expect(count(cmd), cmd).toBeGreaterThan(before[cmd]!);
        expect(sent.slice(mark, told).filter((c) => c.cmd === cmd), `${cmd} before the backend knew`).toEqual([]);
      }
    });

    it("reads nothing again when a choice keeps the language", async () => {
      env.tauri = true;
      await act(async () => root.render(<App />));
      await flush();
      await click("Projekt öffnen");
      await flush();
      await act(async () => atlas("einstellungen").click());
      await flush();
      sent.length = 0;
      await act(async () => languageOption("Deutsch")!.click());
      await flush();
      expect(i18next.language).toBe("de");
      expect(sent.map((c) => c.cmd)).toEqual(["set_locale"]);
    });
  });
});
