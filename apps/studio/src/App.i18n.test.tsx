// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LANGUAGE_KEY } from "./i18n";

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
const invoke = vi.hoisted(() =>
  vi.fn(async (cmd: string) =>
    cmd === "agent_settings" ? { settings: { version: 1, enabled: false, expert: false, grants: [] }, setup_line: "claude mcp add gen4-studio -- \"/x\" mcp", open_project: null } : undefined,
  ),
);
vi.mock("./platform/transport", () => ({ invoke, inTauri: () => false }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ onCloseRequested: async () => () => {} }) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));

import App from "./App";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

describe("App language switch", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(async () => {
    act(() => root.unmount());
    container.remove();
    localStorage.clear();
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
    await flush();

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
    await flush();

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
});
