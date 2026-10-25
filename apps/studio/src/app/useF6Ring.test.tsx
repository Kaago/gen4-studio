// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { f6Targets, useF6Ring } from "./useF6Ring";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** A group of a dock as dockview builds it: in its split view, with its tab strip (the active tab is the one with `tabIndex` 0). */
const group = (id: string, tabs: string[], kind = "") => `
  <div class="dv-view visible">
    <div class="dv-groupview" data-dock-group="${id}" ${kind}>
      <div role="tablist">
        ${tabs.map((t, i) => `<div role="tab" class="dv-tab${i === 0 ? " dv-active-tab" : ""}" tabindex="${i === 0 ? 0 : -1}"><span data-dock-tab="${t}">${t}</span></div>`).join("")}
      </div>
      <div class="dv-content-container" role="tabpanel"></div>
    </div>
  </div>`;

/** A panel's content as dockview renders it with `renderer: "always"`: over its group, not inside it. */
const overlay = (id: string, content: string) => `<div class="dv-render-overlay" tabindex="-1"><div data-dock-panel="${id}">${content}</div></div>`;

/** The frame of the shell: title, atlas, the world (shown) and the texts (hidden), each with list and deck, and the status line. */
const FRAME = `
  <header><button>Projekt öffnen</button></header>
  <nav aria-label="Bereiche"><button data-area="start">Start</button><button data-area="welt" aria-current="page">Welt</button></nav>
  <div data-area="welt" data-part="list">
    <div>
      <section aria-label="Welt" id="welt-list">
        <input aria-label="Orte suchen" id="search" />
        <div role="tree"><div role="treeitem" tabindex="0" id="row">Zweiblattdorf</div><div role="treeitem" tabindex="-1">Route 201</div></div>
        <section aria-label="In dieser Szene"><button>Zurück</button></section>
      </section>
      <button aria-label="Liste einklappen"></button>
    </div>
  </div>
  <main data-area="welt" data-part="deck">
    <div role="toolbar" aria-label="Scharnier" id="hinge"><button id="crumb">Zweiblattdorf</button></div>
    <div id="dock-welt" data-dock="welt">
      <div class="dv-grid-view">
        ${group("1", ["world", "matrix"], 'data-dock-group-kind="schau"')}
        ${group("2", ["script"])}
        ${group("3", ["inspector"])}
      </div>
      ${overlay("world", "<canvas></canvas>")}
      ${overlay("matrix", "<button>Neue Matrix</button>")}
      ${overlay("script", '<button id="save">Speichern</button>')}
      ${overlay("inspector", '<input id="field" /><div role="tree"><div role="treeitem" tabindex="0" id="figure">Figur</div></div>')}
      <div class="dv-floating-overlay-host"><div class="dv-resize-container">${group("4", ["problems"])}</div></div>
      ${overlay("problems", '<button id="fix">Beheben</button>')}
    </div>
  </main>
  <div data-area="texte" data-part="list" hidden><div><section aria-label="Texte"><button>Archiv</button></section></div></div>
  <main data-area="texte" data-part="deck" hidden>
    <div role="toolbar" aria-label="Scharnier"><button>Texte</button></div>
    <div id="dock-texte" data-dock="texte">${group("5", ["textView"])}${overlay("textView", "<button>Weiter</button>")}</div>
  </main>
  <footer><div data-status-line=""><span>Bereit</span><button id="density">Komfort</button></div></footer>
`;

describe("useF6Ring", () => {
  let frame: HTMLElement;
  let container: HTMLElement;
  let root: Root;
  let enabled = true;

  function Probe() {
    useF6Ring(() => enabled);
    return null;
  }
  const $ = (selector: string) => document.querySelector<HTMLElement>(selector)!;
  const groupEl = (id: string) => $(`[data-dock-group="${id}"]`);
  /** F6 (Shift+F6 backwards) where the focus is; whether the key was taken. */
  const f6 = (shiftKey = false) => {
    const event = new KeyboardEvent("keydown", { key: "F6", shiftKey, bubbles: true, cancelable: true });
    act(() => void (document.activeElement ?? document.body).dispatchEvent(event));
    return event.defaultPrevented;
  };
  const focused = () => document.activeElement as HTMLElement;

  beforeEach(() => {
    enabled = true;
    frame = document.createElement("div");
    frame.innerHTML = FRAME;
    document.body.append(frame);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<Probe />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    frame.remove();
    document.querySelectorAll('[role="dialog"]').forEach((d) => d.remove());
  });

  it("f6Targets_lists_atlas_list_hinge_groups_status_in_order", () => {
    expect(f6Targets(document, "welt")).toEqual([$("nav"), $("#welt-list"), $("#hinge"), groupEl("1"), groupEl("2"), groupEl("3"), groupEl("4"), $("footer")]);
  });

  it("puts_the_floating_groups_after_the_docked_ones", () => {
    // A floating group earlier in the document than a docked one still comes last.
    $("#dock-welt").prepend($(".dv-floating-overlay-host"));
    expect(f6Targets(document, "welt").slice(3, 7)).toEqual([groupEl("1"), groupEl("2"), groupEl("3"), groupEl("4")]);
  });

  it("skips_hidden_areas_and_a_folded_list", () => {
    // The texts are hidden: none of their parts is a stop.
    expect(f6Targets(document, "texte")).toEqual([$("nav"), $("footer")]);
    // A folded list and the groups a maximized group hides are no stops either.
    $("#welt-list").hidden = true;
    groupEl("2").closest(".dv-view")!.classList.remove("visible");
    expect(f6Targets(document, "welt")).toEqual([$("nav"), $("#hinge"), groupEl("1"), groupEl("3"), groupEl("4"), $("footer")]);
  });

  it("cycles_forward_and_backward_and_wraps", () => {
    // Atlas: the shown area's entry; list: its roving row, not the search field; the view group: its active tab (the scene has no
    // focusable part); a group with a roving tree: its row; the status line: its first button.
    const forward = ['[data-area="welt"][aria-current]', "#row", "#crumb", '[data-dock-group="1"] .dv-active-tab', "#save", "#figure", "#fix", "#density"];
    for (const selector of forward) {
      expect(f6()).toBe(true);
      expect(focused()).toBe($(selector));
    }
    // Round the end: back to the atlas, and Shift+F6 from the atlas to the status line.
    f6();
    expect(focused()).toBe($('[data-area="welt"][aria-current]'));
    f6(true);
    expect(focused()).toBe($("#density"));
    f6(true);
    expect(focused()).toBe($("#fix"));
  });

  it("jumps_from_a_field_of_the_list_to_the_hinge_and_from_a_panel_to_the_next_group", () => {
    $("#search").focus();
    expect(f6()).toBe(true);
    expect(focused()).toBe($("#crumb"));
    // The focus in a panel's content (rendered over its group, outside it) counts as in that group.
    $("#field").focus();
    f6();
    expect(focused()).toBe($("#fix"));
    $("#save").focus();
    f6(true);
    expect(focused()).toBe($('[data-dock-group="1"] .dv-active-tab'));
  });

  it("focuses_a_stop_with_nothing_focusable_itself", () => {
    $("#density").remove();
    $("#fix").focus();
    f6();
    expect(focused()).toBe($("footer"));
    expect($("footer").tabIndex).toBe(-1);
  });

  it("does_nothing_while_a_modal_dialog_is_open", () => {
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.innerHTML = "<button>OK</button>";
    document.body.append(dialog);
    dialog.querySelector("button")!.focus();
    expect(f6()).toBe(false);
    expect(focused()).toBe(dialog.querySelector("button"));
    dialog.remove();
    // Nor while the caller says no.
    enabled = false;
    $("#search").focus();
    expect(f6()).toBe(false);
    expect(focused()).toBe($("#search"));
  });
});
