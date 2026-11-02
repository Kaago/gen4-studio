// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { readCss, ruleBody } from "../cssSource";
import { Hinge } from "./Hinge";
import type { HingeLive, HingeProps } from "./Hinge";

const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("../../platform/transport", () => ({ invoke: vi.fn(), inTauri: () => env.tauri }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const labels = (n: number) => Array.from({ length: n }, (_, i) => `Schritt ${i + 1}`);

describe("Hinge", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = () => ({ onDepth: vi.fn(), onProblems: vi.fn(), onUndoTo: vi.fn(), onRedo: vi.fn(), onPlay: vi.fn() });
  const base = (over: Partial<HingeProps> = {}): HingeProps => ({
    crumbs: [{ label: "Zweiblattdorf" }, { label: "Zweiblattdorf", tech: "0414" }],
    depth: "bauen",
    problems: { errors: 0, warnings: 0, infos: 0 },
    history: { undo: [], redo: [] },
    playing: false,
    canPlay: true,
    ...handlers(),
    ...over,
  });
  const show = (props: HingeProps) => act(() => root.render(<Hinge {...props} />));
  const bar = () => container.querySelector<HTMLElement>('[role="toolbar"]')!;
  const button = (name: string | RegExp) =>
    [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => {
      const text = b.getAttribute("aria-label") ?? b.textContent ?? "";
      return typeof name === "string" ? text === name || b.textContent === name : name.test(text);
    });
  /** Opens the menu the way a pointer does and returns its items. */
  const openMenu = (trigger: HTMLElement) => {
    act(() => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    return [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];
  };

  beforeEach(() => {
    env.tauri = false;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("is_a_toolbar_called_Scharnier_with_the_crumbs_the_last_one_marked", () => {
    show(base());
    expect(bar().getAttribute("aria-label")).toBe("Scharnier");
    const nav = container.querySelector("nav")!;
    expect(nav.textContent).toContain("Zweiblattdorf");
    expect(nav.textContent).toContain("0414");
    expect(nav.querySelectorAll('[aria-current="location"]')).toHaveLength(1);
    expect(nav.querySelector('[aria-current="location"]')?.textContent).toContain("0414");
  });

  it("gives_every_crumb_its_full_text_as_a_title_so_a_cut_off_crumb_stays_readable", () => {
    const onClick = vi.fn();
    show(base({ crumbs: [{ label: "Zweiblattdorf", onClick }, { label: "Außen 0412", tech: "0394" }, { label: "Warp 0 → Header 0412", tech: "Eventdatei 0394" }] }));
    const titles = [...container.querySelectorAll("nav li[title]")].map((li) => li.getAttribute("title"));
    expect(titles).toEqual(["Zweiblattdorf", "Außen 0412 0394", "Warp 0 → Header 0412 Eventdatei 0394"]);
    // A hover over the button inside a crumb shows the crumb's title.
    expect(button("Zweiblattdorf")!.closest("li")!.getAttribute("title")).toBe("Zweiblattdorf");
  });

  it("lets_the_earlier_crumbs_give_way_first_when_the_bar_is_narrow", () => {
    const css = readCss("./Hinge.module.css", import.meta.url);
    // In a narrow hinge the earlier crumbs shrink first (down to a few letters), so the last one, the selection, stays readable.
    const shrinkOf = (selector: string) => Number(/flex:\s*\S+\s+(\d+)/.exec(ruleBody(css, selector))?.[1] ?? NaN);
    expect(shrinkOf(".crumb")).toBeGreaterThan(shrinkOf(".last"));
    expect(ruleBody(css, ".crumb")).toMatch(/min-width:\s*calc\(var\(--g4-raum-/);
    expect(ruleBody(css, ".last")).toMatch(/min-width:\s*calc\(var\(--g4-raum-/);
    expect(ruleBody(css, ".last")).toMatch(/overflow:\s*hidden/);
    expect(ruleBody(css, ".last")).toMatch(/text-overflow:\s*ellipsis/);
    expect(ruleBody(css, ".crumbButton")).toMatch(/text-overflow:\s*ellipsis/);
  });

  it("makes_a_crumb_with_a_link_a_button_and_one_without_plain_text", () => {
    const onClick = vi.fn();
    show(base({ crumbs: [{ label: "Zweiblattdorf", onClick }, { label: "Kein Link" }] }));
    act(() => button("Zweiblattdorf")!.click());
    expect(onClick).toHaveBeenCalledOnce();
    expect(button("Kein Link")).toBeUndefined();
  });

  it("makes_a_crumb_with_a_menu_a_menu_button_whose_items_switch", () => {
    const onSelect = vi.fn();
    show(base({ crumbs: [{ label: "Pokémon" }, { label: "Pokédex", menu: [{ label: "Pokédex", checked: true, onSelect: vi.fn() }, { label: "Attacken", checked: false, onSelect }] }, { label: "Staravia" }] }));
    const trigger = button("Pokédex")!;
    expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
    const items = [...openMenu(trigger), ...document.querySelectorAll<HTMLElement>("[role=menuitemradio]")];
    const attacks = items.find((i) => i.textContent?.includes("Attacken"))!;
    act(() => attacks.click());
    expect(onSelect).toHaveBeenCalledOnce();
  });

  it("says_what_the_problem_light_counts_and_Alles_stimmig_at_zero", () => {
    show(base({ problems: { errors: 2, warnings: 1, infos: 0 } }));
    expect(button(/2 Fehler, 1 Warnung/)).toBeDefined();
    show(base({ problems: { errors: 0, warnings: 3, infos: 0 } }));
    expect(button(/3 Warnungen/)).toBeDefined();
    show(base());
    expect(button("Alles stimmig")).toBeDefined();
    expect(button("Alles stimmig")!.querySelector("[data-count]")).toBeNull();
  });

  it("says_Nicht_geprueft_instead_of_Alles_stimmig_while_the_checks_have_no_result", () => {
    // No report arrived and none runs (the first run failed or answered without one): nothing is known, the light is no green.
    show(base({ unchecked: true }));
    const light = button("Nicht geprüft")!;
    expect(light.getAttribute("data-light")).toBe("unchecked");
    expect(light.textContent).toBe("Nicht geprüft");
    expect(button("Alles stimmig")).toBeUndefined();
    // Problems of a save or a build are known all the same: counted, and the label says the checks did not run.
    show(base({ unchecked: true, problems: { errors: 1, warnings: 0, infos: 0 } }));
    expect(button("1 Fehler, nicht geprüft")!.getAttribute("data-light")).toBe("error");
    // A running pass says how far it got.
    show(base({ unchecked: true, checking: "Prüft …" }));
    expect(button("Prüft …")).toBeDefined();
  });

  it("shows_three_counters_with_symbols_and_names_them_in_words", () => {
    show(base({ problems: { errors: 2, warnings: 1, infos: 3 } }));
    const light = button("2 Fehler, 1 Warnung, 3 Hinweise")!;
    expect(light.getAttribute("data-light")).toBe("error");
    const counters = [...light.querySelectorAll<HTMLElement>("[data-count]")];
    expect(counters.map((c) => [c.dataset.count, c.textContent])).toEqual([
      ["error", "2"],
      ["warning", "1"],
      ["info", "3"],
    ]);
    // Never colour alone: every counter carries its symbol.
    for (const c of counters) expect(c.querySelector("svg")).not.toBeNull();
    show(base({ problems: { errors: 0, warnings: 0, infos: 1 } }));
    expect(button("1 Hinweis")!.getAttribute("data-light")).toBe("info");
    show(base({ problems: { errors: 0, warnings: 2, infos: 1 } }));
    expect(button("2 Warnungen, 1 Hinweis")!.getAttribute("data-light")).toBe("warning");
    show(base());
    expect(button("Alles stimmig")!.getAttribute("data-light")).toBe("sound");
  });

  it("turns_a_spinner_and_says_the_progress_while_a_pass_runs", () => {
    show(base({ problems: { errors: 1, warnings: 0, infos: 0 }, checking: "Prüft … Texte 312/724" }));
    const light = button(/^Prüft … Texte 312\/724/)!;
    expect(light.getAttribute("aria-label")).toBe("Prüft … Texte 312/724, 1 Fehler");
    expect(light.hasAttribute("data-running")).toBe(true);
    expect(light.textContent).toContain("Prüft … Texte 312/724");
    expect(light.querySelector("[data-count=error]")?.textContent).toBe("1");
    show(base({ checking: "Prüft …" }));
    expect(button("Prüft …")).toBeDefined();
    show(base({ checking: null }));
    expect(button("Alles stimmig")!.hasAttribute("data-running")).toBe(false);
    // The spinner turns, but stands still for someone who asked for less motion.
    const css = readCss("./Hinge.module.css", import.meta.url);
    expect(ruleBody(css, ".spinner")).toMatch(/animation:[^;]*\bspin\b/);
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{\s*\.spinner\s*\{[^}]*animation:\s*none/);
  });

  it("opens_the_problems_when_the_light_is_clicked", () => {
    const props = base({ problems: { errors: 1, warnings: 0, infos: 0 } });
    show(props);
    act(() => button(/1 Fehler/)!.click());
    expect(props.onProblems).toHaveBeenCalledOnce();
  });

  it("switches_the_depth_and_does_nothing_for_the_one_that_is_active", () => {
    const props = base();
    show(props);
    const pressed = () => [...container.querySelectorAll('[aria-pressed="true"]')].map((b) => b.textContent);
    expect(pressed()).toEqual(["Bauen"]);
    act(() => button("Bauen")!.click());
    expect(props.onDepth).not.toHaveBeenCalled();
    act(() => button("Tüfteln")!.click());
    expect(props.onDepth).toHaveBeenCalledExactlyOnceWith("tuefteln");
    show(base({ depth: "tuefteln" }));
    expect(pressed()).toEqual(["Tüfteln"]);
  });

  it("has_three_depths_and_switches_to_erzaehlen", () => {
    const props = base();
    show(props);
    const segment = container.querySelector('[aria-label="Tiefe"]')!;
    expect([...segment.querySelectorAll("button")].map((b) => b.textContent)).toEqual(["Erzählen", "Bauen", "Tüfteln"]);
    expect([...segment.querySelectorAll('[aria-pressed="true"]')].map((b) => b.textContent)).toEqual(["Bauen"]);
    act(() => button("Erzählen")!.click());
    expect(props.onDepth).toHaveBeenCalledExactlyOnceWith("erzaehlen");
    show(base({ depth: "erzaehlen" }));
    expect([...segment.querySelectorAll('[aria-pressed="true"]')].map((b) => b.textContent)).toEqual(["Erzählen"]);
  });

  it("shows_the_number_of_undo_steps_and_lists_the_last_ten_labels_newest_first", () => {
    show(base({ history: { undo: labels(12), redo: [] } }));
    const trigger = button(/Verlauf/)!;
    expect(trigger.textContent).toContain("12");
    const items = openMenu(trigger);
    expect(items.map((i) => i.textContent)).toEqual(["Schritt 12", "Schritt 11", "Schritt 10", "Schritt 9", "Schritt 8", "Schritt 7", "Schritt 6", "Schritt 5", "Schritt 4", "Schritt 3"]);
  });

  it("goes_back_n_steps_when_the_nth_entry_is_clicked", () => {
    const props = base({ history: { undo: labels(5), redo: [] } });
    show(props);
    const items = openMenu(button(/Verlauf/)!);
    act(() => items[2]!.click());
    expect(props.onUndoTo).toHaveBeenCalledExactlyOnceWith(3);
  });

  it("agent_step_shows_client", () => {
    show(base({ history: { undo: ["Malen", "Event hinzugefügt"], redo: [], undo_agents: [null, "Claude Code"], redo_agents: [] } }));
    const [agentStep, ownStep] = openMenu(button(/Verlauf/)!);
    // The newest step comes first: the agent's, with its symbol and the client as quiet detail.
    expect(agentStep?.textContent).toBe("Event hinzugefügtClaude Code");
    expect(agentStep?.querySelector("svg")).not.toBeNull();
    expect(ownStep?.textContent).toBe("Malen");
    expect(ownStep?.querySelector("svg")).toBeNull();
  });

  it("calls_a_step_without_a_label_Aenderung", () => {
    show(base({ history: { undo: ["Tür verschoben", ""], redo: [] } }));
    expect(openMenu(button(/Verlauf/)!).map((i) => i.textContent)).toEqual(["Änderung", "Tür verschoben"]);
  });

  it("offers_nothing_to_go_back_to_while_the_history_is_empty", () => {
    const props = base();
    show(props);
    const items = openMenu(button(/Verlauf/)!);
    expect(items).toHaveLength(1);
    expect(items[0]!.hasAttribute("data-disabled")).toBe(true);
    act(() => items[0]!.click());
    expect(props.onUndoTo).not.toHaveBeenCalled();
  });

  it("disables_redo_while_the_redo_stack_is_empty_and_calls_it_otherwise", () => {
    const props = base();
    show(props);
    expect(button("Wiederholen")!.disabled).toBe(true);
    show({ ...props, history: { undo: ["a"], redo: ["b"] } });
    expect(button("Wiederholen")!.disabled).toBe(false);
    act(() => button("Wiederholen")!.click());
    expect(props.onRedo).toHaveBeenCalledOnce();
  });

  it("plays_with_the_hint_Cmd_R_inside_the_app_and_without_it_in_the_browser", () => {
    const props = base();
    env.tauri = true;
    show(props);
    expect(button(/Hier spielen/)!.textContent).toBe("Hier spielen⌘R");
    act(() => button(/Hier spielen/)!.click());
    expect(props.onPlay).toHaveBeenCalledOnce();
    env.tauri = false;
    show(props);
    expect(button(/Hier spielen/)!.textContent).toBe("Hier spielen");
  });

  it("disables_play_without_a_project_and_says_Baut_while_it_runs", () => {
    show(base({ canPlay: false }));
    expect(button(/Hier spielen/)!.disabled).toBe(true);
    const props = base({ playing: true });
    show(props);
    const playing = button(/Baut/)!;
    expect(playing.textContent).toContain("Baut …");
    // Busy blocks with aria-disabled (keeps focus) and a guard in the click.
    expect(playing.disabled).toBe(false);
    expect(playing.getAttribute("aria-disabled")).toBe("true");
    act(() => playing.click());
    expect(props.onPlay).not.toHaveBeenCalled();
    expect(button(/Hier spielen/)).toBeUndefined();
  });

  it("maximizes_the_view_with_its_button_and_says_so", async () => {
    // Only a hinge over a dock with a view has the button.
    show(base());
    expect(container.querySelector('button[aria-label="Schau maximieren"]')).toBeNull();
    const onMaximize = vi.fn();
    show(base({ maximized: false, onMaximize }));
    const maximize = () => container.querySelector<HTMLButtonElement>('button[aria-label="Schau maximieren"]')!;
    expect(maximize().getAttribute("aria-pressed")).toBe("false");
    // An icon button in the right cluster, just before „Hier spielen“.
    const right = [...maximize().closest("div")!.querySelectorAll("button")].map((b) => b.getAttribute("aria-label") ?? b.textContent);
    expect(right.slice(-2)).toEqual(["Schau maximieren", "Hier spielen"]);
    expect(maximize().textContent).toBe("");
    act(() => maximize().click());
    expect(onMaximize).toHaveBeenCalledOnce();
    // While the view is maximized the button is pressed (a click restores it).
    show(base({ maximized: true, onMaximize }));
    expect(maximize().getAttribute("aria-pressed")).toBe("true");
    act(() => maximize().click());
    expect(onMaximize).toHaveBeenCalledTimes(2);
    // Its tooltip names it.
    vi.useFakeTimers();
    try {
      await act(async () => {
        maximize().dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
        maximize().dispatchEvent(new MouseEvent("mouseenter"));
        maximize().dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
        await vi.advanceTimersByTimeAsync(500);
      });
      expect(document.querySelector("[role=tooltip]")?.textContent).toBe("Schau maximieren");
    } finally {
      vi.useRealTimers();
    }
    // In the wizard's mode the bar shows the wizard only.
    show(base({ maximized: false, onMaximize, wizard: true }));
    expect(container.querySelector('button[aria-label="Schau maximieren"]')).toBeNull();
  });

  it("has_a_hidden_slot_for_the_wizard_bar_only_where_wizards_show", () => {
    show(base());
    expect(container.querySelector("[data-wizard-bar]")).toBeNull();
    show(base({ wizard: false }));
    const slot = container.querySelector<HTMLElement>("[data-wizard-bar]")!;
    expect(slot.hidden).toBe(true);
    expect(container.querySelector("nav")).not.toBeNull();
  });

  it("shows_the_wizard_slot_instead_of_its_content", () => {
    show(base({ wizard: true }));
    const slot = container.querySelector<HTMLElement>("[data-wizard-bar]")!;
    expect(slot.hidden).toBe(false);
    expect(container.querySelector("nav")).toBeNull();
    expect(button(/Hier spielen/)).toBeUndefined();
  });

  it("has_no_grip_and_does_not_resize_on_a_drag", () => {
    show(base());
    expect(container.querySelector("[data-hinge-grip]")).toBeNull();
    expect(button(/Hand ein- oder ausklappen/)).toBeUndefined();
    // The splitter between view and hand sets the height; the bar takes no pointer capture.
    Element.prototype.setPointerCapture = vi.fn();
    const down = new MouseEvent("pointerdown", { bubbles: true, cancelable: true, button: 0, clientY: 580 });
    Object.defineProperty(down, "pointerId", { value: 1 });
    act(() => void bar().dispatchEvent(down));
    expect(Element.prototype.setPointerCapture).not.toHaveBeenCalled();
  });
  describe("in play mode", () => {
    const live = (over: Partial<HingeLive> = {}): HingeLive => ({ state: "live", text: "Live", breakpoints: 0, onResume: vi.fn(), onReload: vi.fn(), onStop: vi.fn(), ...over });
    const liveButton = (attr: string) => container.querySelector<HTMLButtonElement>(`button[${attr}]`);

    it("turns_amber_in_play_mode_and_says_spielt_in_melonds", () => {
      show(base());
      expect(bar().getAttribute("data-hinge-mode")).toBeNull();
      show(base({ live: live({ state: "connecting", text: "Verbindet …" }) }));
      expect(bar().className).toContain("playMode");
      expect(bar().getAttribute("data-hinge-mode")).toBe("play");
      expect(bar().textContent).toContain("Spielt in melonDS");
      expect(container.querySelector("[data-live-state]")?.getAttribute("data-live-state")).toBe("connecting");
      expect(container.querySelector("[data-live-state]")?.textContent).toBe("Verbindet …");
      expect(button(/Hier spielen/)).toBeUndefined();
      // Off is no play mode: the hinge stays dark and offers „Hier spielen“.
      show(base({ live: live({ state: "off" }) }));
      expect(bar().className).not.toContain("playMode");
      expect(bar().getAttribute("data-hinge-mode")).toBeNull();
      expect(button(/Hier spielen/)).toBeDefined();
      // The amber comes from the live theme tokens; the hinge keeps its transition.
      const css = readCss("./Hinge.module.css", import.meta.url);
      const play = ruleBody(css, ".playMode");
      expect(play).toMatch(/--g4-scharnier-grund:\s*var\(--g4-live-grund\)/);
      expect(play).toMatch(/--g4-scharnier-text:\s*var\(--g4-live-auf\)/);
      expect(play).toMatch(/--g4-scharnier-leise:\s*var\(--g4-live-auf\)/);
      // R17: the controls stand on `--g4-live-taste` (readable in both themes), the selection is dark with light text, the lines dark.
      expect(play).toMatch(/--g4-scharnier-taste:\s*var\(--g4-live-taste\)/);
      expect(play).toMatch(/--g4-scharnier-kante:\s*var\(--g4-live-text\)/);
      expect(play).toMatch(/--g4-flaeche-blatt:\s*var\(--g4-live-auf\)/);
      expect(play).toMatch(/--g4-auswahl-flaeche:\s*var\(--g4-live-auf\)/);
      expect(play).toMatch(/--g4-akzent-text:\s*var\(--g4-live-taste\)/);
      expect(play).toMatch(/--g4-linie-standard:\s*var\(--g4-live-auf\)/);
      expect(play).not.toMatch(/#[0-9a-f]{3,8}\b/i);
      expect(ruleBody(css, ".hinge")).toMatch(/transition:\s*background var\(--g4-dauer-scharnier\)/);
    });

    it("offers_weiter_only_while_halted_and_calls_resume", () => {
      show(base({ live: live() }));
      expect(liveButton("data-live-resume")).toBeNull();
      const halted = live({ state: "halted", text: "Gehalten: Figur 3: 0 → 1" });
      show(base({ live: halted }));
      const resume = liveButton("data-live-resume")!;
      expect(resume.textContent).toBe("Weiter");
      expect(resume.getAttribute("data-kind")).toBe("primary");
      expect(container.querySelector("[data-live-state]")?.textContent).toBe("Gehalten: Figur 3: 0 → 1");
      // The one primary button of the bar.
      expect(container.querySelectorAll('button[data-kind="primary"]')).toHaveLength(1);
      act(() => resume.click());
      expect(halted.onResume).toHaveBeenCalledOnce();
    });

    it("neu_laden_and_stoppen_call_their_handlers", () => {
      const props = live({ state: "lost", text: "Verbindung verloren: Port zu" });
      show(base({ live: props }));
      expect(liveButton("data-live-reload")!.textContent).toBe("Neu laden");
      expect(liveButton("data-live-stop")!.textContent).toBe("Stoppen");
      expect(liveButton("data-live-reload")!.getAttribute("data-kind")).toBe("secondary");
      expect(liveButton("data-live-stop")!.getAttribute("data-kind")).toBe("secondary");
      act(() => liveButton("data-live-reload")!.click());
      expect(props.onReload).toHaveBeenCalledOnce();
      act(() => liveButton("data-live-stop")!.click());
      expect(props.onStop).toHaveBeenCalledOnce();
      // While it builds again, „Neu laden“ waits (aria-disabled keeps the focus).
      show(base({ live: props, playing: true }));
      expect(liveButton("data-live-reload")!.getAttribute("aria-disabled")).toBe("true");
      act(() => liveButton("data-live-reload")!.click());
      expect(props.onReload).toHaveBeenCalledOnce();
    });

    it("keeps_weiter_neu_laden_and_stoppen_whole_and_lets_the_state_text_shrink_first", () => {
      // B1: in a narrow hinge the state text ends in an ellipsis (its title carries the full text); the badge and the buttons never shrink.
      const text = "Gehalten: Auslöser 0 · Eventdatei 394: 0 → 1";
      show(base({ live: live({ state: "halted", text }) }));
      const state = container.querySelector<HTMLElement>("[data-live-state]")!;
      expect(state.getAttribute("title")).toBe(text);
      const group = container.querySelector<HTMLElement>("[data-live-buttons]")!;
      for (const attr of ["data-live-resume", "data-live-reload", "data-live-stop"]) expect(group.querySelector(`button[${attr}]`)).not.toBeNull();
      expect(container.querySelector("[data-live-badge]")?.textContent).toBe("Spielt in melonDS");
      const css = readCss("./Hinge.module.css", import.meta.url);
      // The right cluster may shrink (it held its full width before, and the buttons fell off the bar's end).
      expect(ruleBody(css, ".right")).toMatch(/flex:\s*0 1 auto/);
      expect(ruleBody(css, ".right")).toMatch(/min-width:\s*0/);
      expect(ruleBody(css, ".liveActions")).toMatch(/flex:\s*0 1 auto/);
      expect(ruleBody(css, ".liveActions")).toMatch(/min-width:\s*0/);
      const textRule = ruleBody(css, ".liveText");
      expect(textRule).toMatch(/flex:\s*0 1 auto/);
      expect(textRule).toMatch(/min-width:\s*0/);
      expect(textRule).toMatch(/overflow:\s*hidden/);
      expect(textRule).toMatch(/text-overflow:\s*ellipsis/);
      expect(textRule).toMatch(/white-space:\s*nowrap/);
      expect(textRule).not.toMatch(/max-width/);
      expect(ruleBody(css, ".liveButtons")).toMatch(/flex:\s*none/);
      expect(ruleBody(css, ".liveBadge")).toMatch(/flex:\s*none/);
    });

    describe("in a bar too narrow for everything", () => {
      /** jsdom lays nothing out: the bar's widths follow the squeeze it shows, the state text's from the table. */
      let barWidth = 700;
      let barContent: number[] = [];
      let textWidth: number[] = [];
      let observed: (() => void) | null = null;
      const level = () => Number(bar().getAttribute("data-hinge-squeeze") ?? 0);
      const saved = {
        scroll: Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollWidth"),
        client: Object.getOwnPropertyDescriptor(HTMLElement.prototype, "clientWidth"),
        observer: globalThis.ResizeObserver,
      };
      beforeEach(() => {
        Object.defineProperty(HTMLElement.prototype, "scrollWidth", {
          configurable: true,
          get(this: HTMLElement) {
            if (this.getAttribute("aria-label") === "Scharnier") return barContent[level()] ?? barWidth;
            if (this.hasAttribute("data-live-state")) return 300;
            return 0;
          },
        });
        Object.defineProperty(HTMLElement.prototype, "clientWidth", {
          configurable: true,
          get(this: HTMLElement) {
            if (this.getAttribute("aria-label") === "Scharnier") return barWidth;
            if (this.hasAttribute("data-live-state")) return textWidth[level()] ?? 300;
            return 0;
          },
        });
        globalThis.ResizeObserver = class {
          constructor(cb: () => void) {
            observed = cb;
          }
          observe() {}
          unobserve() {}
          disconnect() {}
        } as unknown as typeof ResizeObserver;
      });
      afterEach(() => {
        if (saved.scroll) Object.defineProperty(HTMLElement.prototype, "scrollWidth", saved.scroll);
        if (saved.client) Object.defineProperty(HTMLElement.prototype, "clientWidth", saved.client);
        globalThis.ResizeObserver = saved.observer;
        observed = null;
        barWidth = 700;
        barContent = [];
        textWidth = [];
      });
      const narrow = (over: Partial<HingeProps> = {}) =>
        base({ live: live({ state: "halted", text: "Gehalten: Auslöser 0 · Eventdatei 394: 0 → 1" }), problems: { errors: 1, warnings: 101, infos: 11 }, onMaximize: vi.fn(), ...over });
      const depthLabels = () => [...container.querySelectorAll('[aria-label="Tiefe"] button')].map((b) => [b.getAttribute("aria-label"), b.textContent]);

      it("turns_the_depth_into_symbols_first_when_the_state_text_alone_cannot_make_room", () => {
        barContent = [1000, 690];
        show(narrow());
        expect(level()).toBe(1);
        // The names stay as the accessible names (and tooltips); the light and „Schau maximieren“ stay as they are.
        expect(depthLabels()).toEqual([["Erzählen", ""], ["Bauen", ""], ["Tüfteln", ""]]);
        expect(container.querySelectorAll("[data-count]")).toHaveLength(3);
        expect(button("Schau maximieren")).toBeDefined();
      });

      it("then_shows_the_problem_light_as_its_dot_and_lets_schau_maximieren_yield", () => {
        barContent = [1000, 850, 760, 690];
        show(narrow());
        expect(level()).toBe(3);
        const light = container.querySelector<HTMLElement>("[data-light]")!;
        expect(light.querySelectorAll("[data-count]")).toHaveLength(0);
        // The counts stay in the light's name and its title; a click still opens the problems.
        expect(light.getAttribute("aria-label")).toBe("1 Fehler, 101 Warnungen, 11 Hinweise");
        expect(light.getAttribute("title")).toBe("1 Fehler, 101 Warnungen, 11 Hinweise");
        expect(button("Schau maximieren")).toBeUndefined();
        // History, redo, the badge and the three buttons stay.
        expect(button(/Verlauf/)).toBeDefined();
        expect(button("Wiederholen")).toBeDefined();
        expect(container.querySelector("[data-live-badge]")).not.toBeNull();
        for (const attr of ["data-live-resume", "data-live-reload", "data-live-stop"]) expect(container.querySelector(`button[${attr}]`)).not.toBeNull();
      });

      it("keeps_a_pressed_schau_maximieren_so_the_view_can_be_restored", () => {
        barContent = [1000, 850, 760, 690];
        show(narrow({ maximized: true }));
        expect(level()).toBe(3);
        expect(button("Schau maximieren")?.getAttribute("aria-pressed")).toBe("true");
      });

      it("steps_when_the_state_text_would_shrink_below_a_few_words_even_without_overflow", () => {
        // Nothing overflows, but the state text is down to 40 px of its 300: the depth gives way before it.
        textWidth = [40, 140];
        show(narrow());
        expect(level()).toBe(1);
      });

      it("takes_the_room_back_when_the_bar_grows", () => {
        barContent = [1000, 850, 760, 690];
        show(narrow());
        expect(level()).toBe(3);
        barWidth = 1200;
        act(() => observed?.());
        expect(level()).toBe(0);
        expect(depthLabels()).toEqual([[null, "Erzählen"], [null, "Bauen"], [null, "Tüfteln"]]);
        expect(container.querySelectorAll("[data-count]")).toHaveLength(3);
      });

      it("keeps_the_focus_on_a_depth_toggle_when_the_depth_or_the_state_text_changes", () => {
        barContent = [1000, 690];
        const props = narrow();
        show(props);
        expect(level()).toBe(1);
        const toggle = () => container.querySelector<HTMLButtonElement>('[aria-label="Tiefe"] button[aria-label="Tüfteln"]')!;
        const focused = toggle();
        act(() => focused.focus());
        // The parent answers onDepth with the new depth: no width changes, the toggles stay where they are.
        show({ ...props, depth: "tuefteln" });
        expect(level()).toBe(1);
        expect(toggle()).toBe(focused);
        expect(document.activeElement).toBe(focused);
        // A new state text starts the fit again; the toggles stay mounted through it.
        show({ ...props, depth: "tuefteln", live: live({ state: "live", text: "Live · 1 Haltepunkt" }) });
        expect(level()).toBe(1);
        expect(toggle()).toBe(focused);
        expect(document.activeElement).toBe(focused);
      });

      it("hands_the_focus_to_redo_when_schau_maximieren_steps_aside", () => {
        barContent = [1000, 850, 760, 690];
        const props = narrow({ maximized: true, history: { undo: labels(1), redo: ["x"] } });
        show(props);
        expect(level()).toBe(3);
        const max = button("Schau maximieren")!;
        act(() => max.focus());
        act(() => max.click());
        expect(props.onMaximize).toHaveBeenCalledOnce();
        show({ ...props, maximized: false });
        expect(button("Schau maximieren")).toBeUndefined();
        expect(document.activeElement).toBe(button("Wiederholen"));
        // Without anything to redo (a disabled button takes no focus) the history takes it.
        show({ ...props, maximized: true, history: { undo: labels(1), redo: [] } });
        act(() => button("Schau maximieren")!.focus());
        act(() => button("Schau maximieren")!.click());
        show({ ...props, maximized: false, history: { undo: labels(1), redo: [] } });
        expect(document.activeElement).toBe(button(/Verlauf/));
      });

      it("never_squeezes_outside_play_mode", () => {
        barContent = [1000, 850, 760, 690];
        show(narrow({ live: null }));
        expect(level()).toBe(0);
        expect(bar().hasAttribute("data-hinge-squeeze")).toBe(false);
      });
    });

    it("keeps_depth_light_and_history_in_play_mode", () => {
      const props = base({ live: live({ text: "Live · 2 Haltepunkte", breakpoints: 2 }), history: { undo: labels(2), redo: ["x"] } });
      show(props);
      expect(container.querySelector("nav")?.textContent).toContain("Zweiblattdorf");
      expect(container.querySelector('[role="radiogroup"], [aria-label="Tiefe"]')).not.toBeNull();
      expect(container.querySelector("[data-light]")).not.toBeNull();
      expect(button(/Verlauf, 2 Schritte/)).toBeDefined();
      act(() => button("Wiederholen")!.click());
      expect(props.onRedo).toHaveBeenCalledOnce();
      expect(container.querySelector("[data-live-state]")?.textContent).toBe("Live · 2 Haltepunkte");
    });
  });
});
