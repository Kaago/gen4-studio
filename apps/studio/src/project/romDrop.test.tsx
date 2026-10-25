// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { isRomPath, pathFromDataTransfer, useRomDrop } from "./romDrop";
import type { StartWizardRequest } from "./romDrop";

const env = vi.hoisted(() => ({ tauri: false, handler: null as null | ((e: { payload: unknown }) => void), unlistened: 0 }));
vi.mock("../platform/transport", () => ({ invoke: vi.fn(), inTauri: () => env.tauri }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (handler: (e: { payload: unknown }) => void) => {
      env.handler = handler;
      return () => {
        env.unlistened++;
        env.handler = null;
      };
    },
  }),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** A DataTransfer as a browser hands it to `drop` (jsdom has none): the given types with their data, and files by name. */
function transfer(data: Record<string, string>, files: string[] = []): DataTransfer {
  const types = [...Object.keys(data), ...(files.length > 0 ? ["Files"] : [])];
  return { types, getData: (type: string) => data[type] ?? "", files: Object.assign(files.map((name) => ({ name })), { item: (i: number) => ({ name: files[i] }) }) } as unknown as DataTransfer;
}

describe("pathFromDataTransfer", () => {
  it("decodes_a_file_uri_with_spaces", () => {
    expect(pathFromDataTransfer(transfer({ "text/uri-list": "file:///a%20b/x.nds" }, ["x.nds"]))).toEqual({ path: "/a b/x.nds" });
  });

  it("skips_the_comments_of_a_uri_list_and_keeps_a_windows_drive", () => {
    expect(pathFromDataTransfer(transfer({ "text/uri-list": "# from Finder\r\nfile:///C:/Roms/pt.nds\r\n" }))).toEqual({ path: "C:/Roms/pt.nds" });
  });

  it("takes_an_absolute_path_from_plain_text", () => {
    expect(pathFromDataTransfer(transfer({ "text/plain": "  /roms/_testkopien/pt.nds\n" }))).toEqual({ path: "/roms/_testkopien/pt.nds" });
    // Not a path: a word, a relative path.
    expect(pathFromDataTransfer(transfer({ "text/plain": "pt.nds" }))).toEqual({ none: true });
  });

  it("names_a_file_without_a_path", () => {
    expect(pathFromDataTransfer(transfer({}, ["pt.nds"]))).toEqual({ file: "pt.nds" });
  });

  it("refuses_several_files_or_paths_and_nothing", () => {
    expect(pathFromDataTransfer(transfer({}, ["a.nds", "b.nds"]))).toEqual({ none: true });
    expect(pathFromDataTransfer(transfer({ "text/uri-list": "file:///a.nds\nfile:///b.nds" }))).toEqual({ none: true });
    expect(pathFromDataTransfer(transfer({ "text/plain": "/a.nds\n/b.nds" }))).toEqual({ none: true });
    expect(pathFromDataTransfer(transfer({ "text/uri-list": "https://example.org/x.nds" }))).toEqual({ none: true });
    expect(pathFromDataTransfer(transfer({}))).toEqual({ none: true });
  });
});

describe("isRomPath", () => {
  it("knows_the_nds_ending_in_any_case", () => {
    expect(isRomPath("/roms/X.NDS")).toBe(true);
    expect(isRomPath("X.nds")).toBe(true);
    expect(isRomPath("/roms/x.nds.txt")).toBe(false);
    expect(isRomPath("/roms/nds")).toBe(false);
  });
});

describe("useRomDrop in the app", () => {
  let container: HTMLElement;
  let root: Root;
  const onRom = vi.fn<(r: StartWizardRequest) => void>();
  const onRefuse = vi.fn<(msg: string) => void>();
  function Probe({ enabled }: { enabled: boolean }) {
    const { over } = useRomDrop(enabled, onRom, onRefuse);
    return <span data-over={over ? "over" : "idle"} />;
  }
  const over = () => container.querySelector("span")?.dataset.over;
  const fire = (payload: unknown) => act(async () => env.handler?.({ payload }));

  beforeEach(() => {
    env.tauri = true;
    env.handler = null;
    env.unlistened = 0;
    onRom.mockReset();
    onRefuse.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    env.tauri = false;
  });

  it("listens_to_the_webview_and_opens_the_wizard_with_a_dropped_rom", async () => {
    await act(async () => root.render(<Probe enabled />));
    expect(env.handler).not.toBeNull();
    await fire({ type: "enter", paths: ["/roms/pt.nds"], position: { x: 1, y: 1 } });
    expect(over()).toBe("over");
    await fire({ type: "drop", paths: ["/roms/pt.nds"], position: { x: 1, y: 1 } });
    expect(over()).toBe("idle");
    expect(onRom).toHaveBeenCalledExactlyOnceWith({ rom: "/roms/pt.nds" });
  });

  it("refuses_two_files_or_another_file", async () => {
    await act(async () => root.render(<Probe enabled />));
    await fire({ type: "drop", paths: ["/roms/a.nds", "/roms/b.nds"], position: { x: 1, y: 1 } });
    await fire({ type: "drop", paths: ["/roms/notes.txt"], position: { x: 1, y: 1 } });
    expect(onRefuse.mock.calls).toEqual([["Nur eine .nds-Datei ablegen."], ["Nur eine .nds-Datei ablegen."]]);
    expect(onRom).not.toHaveBeenCalled();
    await fire({ type: "enter", paths: ["/roms/a.nds"], position: { x: 1, y: 1 } });
    await fire({ type: "leave" });
    expect(over()).toBe("idle");
  });

  it("stops_listening_while_disabled", async () => {
    await act(async () => root.render(<Probe enabled />));
    await act(async () => root.render(<Probe enabled={false} />));
    expect(env.unlistened).toBe(1);
    expect(env.handler).toBeNull();
  });
});
