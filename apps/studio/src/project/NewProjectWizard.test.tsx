// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { checkProjectDest, createProject, defaultProjectParent, inspectRom } from "../api";
import type { Created, CreateProgress, DestCheck, InitReport, RomInfo } from "../api";
import { initReportOf } from "../git/gitTesting";
import { notInstalledText } from "../git/gitWords";
import { openPath } from "../platform/dialogs";
import { pressKey } from "../ui/primitives/testUtils";
import { NewProjectWizard } from "./NewProjectWizard";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), inspectRom: vi.fn(), defaultProjectParent: vi.fn(), checkProjectDest: vi.fn(), createProject: vi.fn() }));
vi.mock("../platform/dialogs", async (importOriginal) => ({ ...(await importOriginal<typeof import("../platform/dialogs")>()), openPath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
/** Longer than the 200 ms the destination check waits for the typing to stop. */
const settle = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 260)));

const SIZE = 134_217_728;
const rom = (change: Partial<RomInfo> = {}): RomInfo => ({
  title: "POKEMON PL",
  game_code: "CPUD",
  family: "pt",
  language: "german",
  revision: 0,
  size: SIZE,
  sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4",
  known_base: "Platin DE (Testprojekt)",
  support: { kind: "verified" },
  ...change,
});
const PLATIN = rom();
const DIAMANT = rom({ title: "POKEMON D", game_code: "ADAD", family: "dp", known_base: null, support: { kind: "locked", reason: "Diamant/Perl: ohne Testprojekt nicht freigegeben" } });
const HG_FR = rom({ title: "POKEMON HG", game_code: "IPKF", family: "hgss", language: "french", known_base: null, support: { kind: "untested" } });
const free = (change: Partial<DestCheck> = {}): DestCheck => ({ state: "missing", free_bytes: 50_000_000_000, needed_bytes: SIZE * 2.5, ok: true, ...change });
const created = (dest: string, template: Created["template"] = "original"): Created => ({
  root: dest,
  game_code: "CPUD",
  template,
  sha1: PLATIN.sha1,
  phases: [
    { phase: "read", ms: 30 },
    { phase: "extract", ms: 900 },
    { phase: "template", ms: 0 },
    { phase: "check", ms: 40 },
  ],
});

describe("NewProjectWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const onCancel = vi.fn();
  const onOpen = vi.fn();
  const onChoice = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const fn of [onCancel, onOpen, onChoice]) fn.mockReset();
    vi.mocked(inspectRom).mockReset().mockResolvedValue(PLATIN);
    vi.mocked(defaultProjectParent).mockReset().mockResolvedValue({ parent: "/roms/_testkopien", dspre_suffix: false });
    vi.mocked(checkProjectDest).mockReset().mockResolvedValue(free());
    // The browser's way: the phases arrive done, after the work.
    vi.mocked(createProject)
      .mockReset()
      .mockImplementation(async (_rom, dest, template, _textProject, _git, onProgress) => {
        const made = created(dest, template);
        for (const { phase } of made.phases) onProgress({ phase, done: 1, total: 1 });
        return made;
      });
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(props: { initialRom?: string; initialNotice?: string } = {}) {
    await act(async () => root.render(<NewProjectWizard {...props} onCancel={onCancel} onOpen={onOpen} onChoice={onChoice} />));
    await flush();
  }
  const text = () => container.textContent ?? "";
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const next = () => click(button("Weiter"));
  const field = (label: string) => container.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");
  const card = (template: string) => container.querySelector<HTMLElement>(`[role="radio"][data-template="${template}"]`);
  const phase = (id: string) => container.querySelector<HTMLElement>(`[data-check="${id}"]`);
  async function type(input: HTMLInputElement, value: string) {
    await act(async () => input.focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => input.blur());
    await flush();
  }
  /** ROM, template "Original", then on to the step "Name und Ziel" with the destination checked. */
  async function toDestination() {
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    await next();
    await settle();
  }

  it("walks_rom_template_name_and_summary_then_creates", async () => {
    await show();
    expect(current()).toBe("1. ROM");
    expect(text()).toContain("Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM. Die ROM bleibt unverändert.");
    expect(button("Weiter")?.disabled).toBe(true);

    await type(field("ROM-Datei"), "/roms/pt.nds");
    expect(inspectRom).toHaveBeenCalledWith("/roms/pt.nds");
    expect(container.querySelector('[aria-label="DS-Bildschirm, Seite 1 von 1"]')?.textContent).toContain("POKEMON PL");
    for (const shown of ["Platin", "Deutsch", "CPUD", "Freigegeben", "Bekannte Basis-ROM: Platin DE (Testprojekt)"]) expect(text()).toContain(shown);
    await next();

    expect(current()).toBe("2. Vorlage");
    expect(card("original")?.getAttribute("aria-checked")).toBe("true");
    expect(card("original")?.textContent).toContain("Das Spiel, wie es ist.");
    expect(card("data_only")?.textContent).toContain("Für Balancing: Werte, Lernsets, Trainer. Startet im Bereich Pokémon.");
    expect(card("empty_region")?.textContent).toContain("Nur die Startstadt mit Spielerhaus bleibt, der Rest ist eine leere Region. Ein Start-Script gibt dir deinen ersten Partner.");
    // A ROM the editor knows needs no warning on the cards.
    expect(text()).not.toContain("Nicht geprüft");
    await next();

    expect(current()).toBe("3. Name und Ziel");
    expect(field("Name").value).toBe("Neues Platin");
    expect(field("Ordner").value).toBe("/roms/_testkopien");
    expect(container.querySelector('[role="switch"]')?.getAttribute("aria-checked")).toBe("false");
    await settle();
    expect(checkProjectDest).toHaveBeenLastCalledWith("/roms/_testkopien/Neues Platin", SIZE);
    expect(text()).toContain("/roms/_testkopien/Neues Platin");
    expect(onChoice).toHaveBeenLastCalledWith({ rom: PLATIN, template: "original", name: "Neues Platin", dest: "/roms/_testkopien/Neues Platin", textProject: false, git: false });
    // Enter in a field that holds no typed text moves on.
    await act(async () => field("Name").focus());
    pressKey(field("Name"), "Enter");
    await flush();

    expect(current()).toBe("4. Zusammenfassung");
    expect(text()).toContain("Legt „Neues Platin“ aus Platin (Deutsch) mit der Vorlage Original an, in /roms/_testkopien/Neues Platin. Die ROM bleibt unverändert.");
    await click(button("Anlegen"));
    await flush();

    expect(createProject).toHaveBeenCalledWith("/roms/pt.nds", "/roms/_testkopien/Neues Platin", "original", false, false, expect.any(Function));
    expect(text()).toContain("Projekt angelegt");
    expect(text()).toContain("Basis-ROM SHA-1 d40f19ac…");
    expect(text()).toContain("Öffnen lädt das Projekt.");
    expect(button("Hier spielen")).toBeUndefined();
    await click(button("Öffnen"));
    expect(onOpen).toHaveBeenCalledExactlyOnceWith(created("/roms/_testkopien/Neues Platin"));
  });

  it("text_project_checkbox_is_off_and_passes_through_with_its_phase", async () => {
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    expect(current()).toBe("2. Vorlage");
    const box = () => [...container.querySelectorAll<HTMLElement>("[role=checkbox]")].find((b) => b.closest("label")?.textContent === "Als Textprojekt einrichten");
    expect(box()?.getAttribute("aria-checked")).toBe("false");
    expect(text()).toContain("Eine lesbare Datei je Eintrag, bereit für Git; das Anlegen dauert etwas länger.");
    // Off: no phase for it in the summary.
    await next();
    await settle();
    await next();
    expect(phase("text_project")).toBeNull();
    expect(text()).not.toContain("als Textprojekt");
    // On: the summary says so and lists the phase after „Prüfen“.
    await click(button("Zurück"));
    await click(button("Zurück"));
    await click(box());
    expect(box()?.getAttribute("aria-checked")).toBe("true");
    await next();
    await settle();
    await next();
    expect(text()).toContain("Original, als Textprojekt");
    expect(text()).toContain("mit der Vorlage Original an, als Textprojekt, in /roms/_testkopien/Neues Platin.");
    const ids = [...container.querySelectorAll<HTMLElement>("[data-check]")].map((r) => r.dataset.check);
    expect(ids).toEqual(["read", "extract", "template", "check", "text_project"]);
    expect(phase("text_project")?.textContent).toContain("Textprojekt einrichten");
    let progress: (p: CreateProgress) => void = () => {};
    let finish: () => void = () => {};
    vi.mocked(createProject).mockImplementationOnce((_rom, dest, template, _textProject, _git, onProgress) => {
      progress = onProgress;
      return new Promise<Created>((resolve) => (finish = () => resolve(created(dest, template))));
    });
    await click(button("Anlegen"));
    await act(async () => {
      progress({ phase: "check", done: 1, total: 1 });
      progress({ phase: "text_project", done: 3, total: 10 });
    });
    expect(phase("check")?.dataset.phase).toBe("done");
    expect(phase("text_project")?.dataset.phase).toBe("running");
    expect(phase("text_project")?.textContent).toContain("läuft 3/10");
    await act(async () => finish());
    await flush();
    expect(createProject).toHaveBeenCalledWith("/roms/pt.nds", "/roms/_testkopien/Neues Platin", "original", true, false, expect.any(Function));
    expect(text()).toContain("Projekt angelegt");
  });

  it("git_checkbox_is_off_and_passes_through_with_its_phase", async () => {
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    expect(current()).toBe("2. Vorlage");
    const box = (label: string) => [...container.querySelectorAll<HTMLElement>("[role=checkbox]")].find((b) => b.closest("label")?.textContent === label);
    expect(box("Git einrichten (nur lokal)")?.getAttribute("aria-checked")).toBe("false");
    expect(text()).toContain("Legt ein lokales Repository mit erstem Commit an; nichts wird hochgeladen.");
    // It stands under „Als Textprojekt einrichten“.
    const labels = [...container.querySelectorAll<HTMLElement>("[role=checkbox]")].map((b) => b.closest("label")?.textContent);
    expect(labels).toEqual(["Als Textprojekt einrichten", "Git einrichten (nur lokal)"]);
    // Off: neither the summary nor the phases name it.
    await next();
    await settle();
    await next();
    expect(phase("git")).toBeNull();
    expect(text()).not.toContain("mit Git");
    // On, with the text project: the summary says so and lists the phase after „Textprojekt einrichten“.
    await click(button("Zurück"));
    await click(button("Zurück"));
    await click(box("Als Textprojekt einrichten"));
    await click(box("Git einrichten (nur lokal)"));
    expect(box("Git einrichten (nur lokal)")?.getAttribute("aria-checked")).toBe("true");
    await next();
    await settle();
    await next();
    expect(text()).toContain("Original, als Textprojekt, mit Git");
    expect(text()).toContain("mit der Vorlage Original an, als Textprojekt, mit Git, in /roms/_testkopien/Neues Platin.");
    const ids = [...container.querySelectorAll<HTMLElement>("[data-check]")].map((r) => r.dataset.check);
    expect(ids).toEqual(["read", "extract", "template", "check", "text_project", "git"]);
    expect(phase("git")?.textContent).toContain("Git einrichten");
    let progress: (p: CreateProgress) => void = () => {};
    let finish: () => void = () => {};
    vi.mocked(createProject).mockImplementationOnce((_rom, dest, template, _textProject, _git, onProgress) => {
      progress = onProgress;
      return new Promise<Created>((resolve) => (finish = () => resolve(created(dest, template))));
    });
    await click(button("Anlegen"));
    await act(async () => progress({ phase: "git", done: 0, total: 5 }));
    expect(phase("text_project")?.dataset.phase).toBe("done");
    expect(phase("git")?.dataset.phase).toBe("running");
    await act(async () => finish());
    await flush();
    expect(createProject).toHaveBeenCalledWith("/roms/pt.nds", "/roms/_testkopien/Neues Platin", "original", true, true, expect.any(Function));
    expect(text()).toContain("Projekt angelegt");

    // Git without the text project (a fresh wizard): the phase follows „Prüfen“.
    act(() => root.unmount());
    root = createRoot(container);
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    await click(box("Git einrichten (nur lokal)"));
    await next();
    await settle();
    await next();
    expect(text()).toContain("mit der Vorlage Original an, mit Git, in /roms/_testkopien/Neues Platin.");
    expect([...container.querySelectorAll<HTMLElement>("[data-check]")].map((r) => r.dataset.check)).toEqual(["read", "extract", "template", "check", "git"]);
  });

  /** Creates with „Git einrichten (nur lokal)“ on; the backend answers `git` as given. The result step's text and its warning (or null). */
  async function createWithGit(git: InitReport): Promise<{ text: string; warning: string | null }> {
    vi.mocked(createProject).mockImplementationOnce(async (_rom, dest, template) => ({ ...created(dest, template), git }));
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    const box = [...container.querySelectorAll<HTMLElement>("[role=checkbox]")].find((b) => b.closest("label")?.textContent === "Git einrichten (nur lokal)");
    await click(box);
    await next();
    await settle();
    await next();
    await click(button("Anlegen"));
    await flush();
    expect(text()).toContain("Projekt angelegt");
    const warning = [...container.querySelectorAll<HTMLElement>("[role=status]")].find((n) => n.querySelector("svg") !== null && !n.textContent?.includes("Projekt angelegt"));
    return { text: text(), warning: warning?.textContent ?? null };
  }

  it("the_result_names_the_first_commit_and_a_studio_author", async () => {
    const plain = await createWithGit(initReportOf());
    expect(plain.text).toContain("Git eingerichtet: 18.449 Dateien im ersten Commit „Ausgangsstand“.");
    expect(plain.text).not.toContain("Autor ist");
    expect(plain.warning).toBeNull();
    act(() => root.unmount());
    root = createRoot(container);
    const studio = await createWithGit(initReportOf({ studio_identity: true }));
    expect(studio.text).toContain("Git eingerichtet: 18.449 Dateien im ersten Commit „Ausgangsstand“. Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.");
    expect(studio.warning).toBeNull();
  });

  it("the_result_says_only_the_templates_were_written_without_git", async () => {
    const { text: shown, warning } = await createWithGit(initReportOf({ committed: false, reason: notInstalledText(), head: null, files: 0 }));
    expect(warning).toBe("Vorlagen geschrieben. Git ist nicht installiert; das Repository legst du an, sobald Git da ist.");
    expect(shown).not.toContain("Git eingerichtet");
  });

  it("the_result_shows_why_the_first_commit_failed", async () => {
    const reason = "Der erste Commit ist gescheitert: error: gpg failed to sign the data. Repository und Vorlagen sind angelegt; committe mit deinem Git-Werkzeug.";
    const { text: shown, warning } = await createWithGit(initReportOf({ committed: false, reason, head: null, files: 0 }));
    expect(warning).toBe(reason);
    expect(shown).not.toContain("Git eingerichtet");
  });

  it("creates_with_the_dspre_ending_and_only_data", async () => {
    vi.mocked(defaultProjectParent).mockResolvedValue({ parent: "/Users/a/Documents/Gen4 Studio/", dspre_suffix: true });
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    await click(card("data_only"));
    expect(card("data_only")?.getAttribute("aria-checked")).toBe("true");
    expect(card("original")?.getAttribute("aria-checked")).toBe("false");
    await next();
    await settle();
    expect(container.querySelector('[role="switch"]')?.getAttribute("aria-checked")).toBe("true");
    expect(checkProjectDest).toHaveBeenLastCalledWith("/Users/a/Documents/Gen4 Studio/Neues Platin_DSPRE_contents", SIZE);
    await next();
    expect(text()).toContain("mit der Vorlage Nur Daten an, in /Users/a/Documents/Gen4 Studio/Neues Platin_DSPRE_contents.");
    await click(button("Anlegen"));
    await flush();
    expect(createProject).toHaveBeenCalledWith("/roms/pt.nds", "/Users/a/Documents/Gen4 Studio/Neues Platin_DSPRE_contents", "data_only", false, false, expect.any(Function));
    expect(text()).toContain("Öffnen startet im Bereich Pokémon.");
  });

  it("locks_diamond_with_its_reason", async () => {
    vi.mocked(inspectRom).mockResolvedValue(DIAMANT);
    await show();
    await type(field("ROM-Datei"), "/roms/d.nds");
    expect(text()).toContain("Diamant/Perl");
    expect(text()).toContain("Gesperrt");
    expect(text()).toContain("Diamant/Perl: ohne Testprojekt nicht freigegeben");
    expect(text()).toContain("Unbekannte Fassung: vielleicht schon verändert.");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("names_a_file_that_is_no_gen4_game", async () => {
    vi.mocked(inspectRom).mockResolvedValue(rom({ title: "TETRIS", game_code: "ATRE", family: null, language: "english", known_base: null, support: { kind: "unknown", reason: "Kein Gen-4-Spiel erkannt" } }));
    await show();
    await type(field("ROM-Datei"), "/roms/t.nds");
    expect(text()).toContain("Kein Gen-4-Spiel erkannt");
    expect(button("Weiter")?.disabled).toBe(true);
    // A file that is no ROM at all: the refusal shows, and the card of the earlier file goes.
    vi.mocked(inspectRom).mockRejectedValue({ code: "error", message: "too small for a DS ROM: 12 bytes", asset: null });
    await type(field("ROM-Datei"), "/roms/leer.nds");
    expect(text()).toContain("too small for a DS ROM: 12 bytes");
    expect(text()).not.toContain("TETRIS");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("untested_language_shows_the_plaque", async () => {
    vi.mocked(inspectRom).mockResolvedValue(HG_FR);
    await show();
    await type(field("ROM-Datei"), "/roms/hg-fr.nds");
    expect(text()).toContain("HeartGold");
    expect(text()).toContain("Französisch");
    expect(text()).toContain("Nicht geprüft");
    expect(text()).toContain("Für diese Sprache gibt es kein Testprojekt. Entpacken geht, Editoren und Vorlagen sind nicht geprüft.");
    expect(button("Weiter")?.disabled).toBe(false);
    await next();
    for (const t of ["original", "empty_region", "data_only"]) expect(card(t)?.textContent).toContain("Nicht geprüft");
    await next();
    expect(field("Name").value).toBe("Neues HeartGold");
  });

  it("refuses_a_non_empty_destination", async () => {
    vi.mocked(checkProjectDest).mockResolvedValue(free({ state: "not_empty", ok: false }));
    await show();
    await toDestination();
    expect(text()).toContain("Den Ordner gibt es schon und er ist nicht leer. Wähle einen anderen Namen.");
    expect(button("Weiter")?.disabled).toBe(true);
    // Another name is free.
    vi.mocked(checkProjectDest).mockResolvedValue(free());
    await type(field("Name"), "Mein Platin");
    await settle();
    expect(checkProjectDest).toHaveBeenLastCalledWith("/roms/_testkopien/Mein Platin", SIZE);
    expect(text()).not.toContain("Den Ordner gibt es schon");
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("refuses_too_little_space_a_bad_name_and_a_rejected_check", async () => {
    vi.mocked(checkProjectDest).mockResolvedValue(free({ free_bytes: 100_000_000, needed_bytes: 335_544_320, ok: false }));
    await show();
    await toDestination();
    expect(text()).toContain("Zu wenig Platz: 100 MB frei, gebraucht etwa 336 MB.");
    expect(button("Weiter")?.disabled).toBe(true);

    vi.mocked(checkProjectDest).mockResolvedValue(free());
    await type(field("Name"), "a/b");
    await settle();
    expect(text()).toContain("Der Name darf kein /, \\ oder : enthalten und nicht mit einem Punkt beginnen.");
    expect(button("Weiter")?.disabled).toBe(true);

    // The dev bridge refuses a destination outside its roots.
    vi.mocked(checkProjectDest).mockRejectedValue({ code: "error", message: "Pfad liegt außerhalb der erlaubten Ordner: /tmp/Neu", asset: null });
    await type(field("Name"), "Neu");
    await type(field("Ordner"), "/tmp");
    await settle();
    expect(text()).toContain("Pfad liegt außerhalb der erlaubten Ordner: /tmp/Neu");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("initial_rom_jumps_to_the_template_step", async () => {
    await show({ initialRom: "/roms/pt.nds" });
    expect(inspectRom).toHaveBeenCalledWith("/roms/pt.nds");
    expect(current()).toBe("2. Vorlage");
  });

  it("an_initial_untested_rom_stays_on_the_rom_step_so_its_warning_is_read", async () => {
    vi.mocked(inspectRom).mockResolvedValue(HG_FR);
    await show({ initialRom: "/roms/hg-fr.nds" });
    expect(inspectRom).toHaveBeenCalledWith("/roms/hg-fr.nds");
    expect(current()).toBe("1. ROM");
    expect(text()).toContain("Für diese Sprache gibt es kein Testprojekt. Entpacken geht, Editoren und Vorlagen sind nicht geprüft.");
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("the_arrow_keys_move_the_template_choice_and_wrap", async () => {
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    await act(async () => card("original")!.focus());
    pressKey(card("original")!, "ArrowRight");
    await flush();
    expect(card("empty_region")?.getAttribute("aria-checked")).toBe("true");
    expect(card("original")?.getAttribute("aria-checked")).toBe("false");
    expect(document.activeElement).toBe(card("empty_region"));
    // Only the chosen card is a tab stop.
    expect(card("empty_region")?.tabIndex).toBe(0);
    expect(card("original")?.tabIndex).toBe(-1);
    pressKey(card("empty_region")!, "ArrowDown");
    await flush();
    expect(card("data_only")?.getAttribute("aria-checked")).toBe("true");
    expect(document.activeElement).toBe(card("data_only"));
    pressKey(card("data_only")!, "ArrowDown");
    await flush();
    expect(card("original")?.getAttribute("aria-checked")).toBe("true");
    expect(document.activeElement).toBe(card("original"));
    pressKey(card("original")!, "ArrowLeft");
    await flush();
    expect(card("data_only")?.getAttribute("aria-checked")).toBe("true");
    expect(document.activeElement).toBe(card("data_only"));
    pressKey(card("data_only")!, "ArrowUp");
    await flush();
    expect(card("empty_region")?.getAttribute("aria-checked")).toBe("true");
  });

  it("empty_region_card_is_selectable", async () => {
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    expect(card("empty_region")?.getAttribute("aria-disabled")).toBeNull();
    expect(card("empty_region")?.textContent).not.toContain("Kommt mit");
    await click(card("empty_region"));
    expect(card("empty_region")?.getAttribute("aria-checked")).toBe("true");
    expect(card("original")?.getAttribute("aria-checked")).toBe("false");
    await next();
    await settle();
    await next();
    expect(text()).toContain("Legt „Neues Platin“ aus Platin (Deutsch) mit der Vorlage Leere Region an, in /roms/_testkopien/Neues Platin. Scripts werden übernommen. Die ROM bleibt unverändert.");
    await click(button("Anlegen"));
    await flush();
    expect(createProject).toHaveBeenCalledWith("/roms/pt.nds", "/roms/_testkopien/Neues Platin", "empty_region", false, false, expect.any(Function));
    expect(text()).toContain("Projekt angelegt");
  });

  it("a_picked_folder_clears_the_error_of_the_default_folder", async () => {
    vi.mocked(defaultProjectParent).mockRejectedValue({ code: "error", message: "Kein Ordner für Projekte", asset: null });
    vi.mocked(openPath).mockResolvedValue("/roms/_testkopien");
    await show();
    await type(field("ROM-Datei"), "/roms/pt.nds");
    await next();
    await next();
    expect(text()).toContain("Kein Ordner für Projekte");
    await click(button("Ordner wählen…"));
    await flush();
    expect(field("Ordner").value).toBe("/roms/_testkopien");
    expect(text()).not.toContain("Kein Ordner für Projekte");
    await settle();
    expect(button("Weiter")?.disabled).toBe(false);
  });

  it("checks_the_destination_again_after_a_failed_create", async () => {
    await show();
    await toDestination();
    await next();
    const checks = vi.mocked(checkProjectDest).mock.calls.length;
    // The work failed and left something behind: the check must not keep its old answer.
    vi.mocked(createProject).mockRejectedValue({ code: "error", message: "build: boom", asset: null });
    vi.mocked(checkProjectDest).mockResolvedValue(free({ state: "not_empty", ok: false }));
    await click(button("Anlegen"));
    await flush();
    await settle();
    expect(vi.mocked(checkProjectDest).mock.calls.length).toBeGreaterThan(checks);
    await click(button("Zurück"));
    expect(current()).toBe("3. Name und Ziel");
    expect(text()).toContain("Den Ordner gibt es schon und er ist nicht leer. Wähle einen anderen Namen.");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("shows_the_initial_notice_on_the_rom_step", async () => {
    await show({ initialNotice: "Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein." });
    expect(current()).toBe("1. ROM");
    expect(text()).toContain("Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein.");
  });

  it("shows_progress_and_an_error", async () => {
    let fail: (e: unknown) => void = () => {};
    let progress: (p: CreateProgress) => void = () => {};
    // The Tauri way: each event says a phase starts.
    vi.mocked(createProject).mockImplementation((_rom, _dest, _template, _textProject, _git, onProgress) => {
      progress = onProgress;
      onProgress({ phase: "read", done: 0, total: 1 });
      return new Promise<Created>((_, reject) => (fail = reject));
    });
    await show();
    await toDestination();
    await next();
    expect(current()).toBe("4. Zusammenfassung");
    for (const id of ["read", "extract", "template", "check"]) expect(phase(id)?.dataset.phase).toBe("open");
    expect(phase("read")?.textContent).toContain("Lesen");
    expect(phase("extract")?.textContent).toContain("Entpacken");
    expect(phase("template")?.textContent).toContain("Vorlage anwenden");
    expect(phase("check")?.textContent).toContain("Prüfen");

    await click(button("Anlegen"));
    expect(container.querySelector('[role="dialog"]')?.getAttribute("aria-busy")).toBe("true");
    expect(phase("read")?.dataset.phase).toBe("running");
    expect(phase("extract")?.dataset.phase).toBe("open");
    await act(async () => progress({ phase: "extract", done: 0, total: 1 }));
    expect(phase("read")?.dataset.phase).toBe("done");
    expect(phase("extract")?.dataset.phase).toBe("running");

    await act(async () => fail({ code: "error", message: "build: boom", asset: null }));
    await flush();
    expect(container.querySelector('[role="alert"]')?.textContent).toBe("build: boom");
    expect(phase("extract")?.dataset.phase).toBe("failed");
    expect(current()).toBe("4. Zusammenfassung");
    // The destination is asked again after the failure; once it answers, the work can be retried.
    await settle();
    expect(button("Anlegen")?.disabled).toBe(false);

    // A known refusal of the kernel reads in German; going back leaves the error behind.
    vi.mocked(createProject).mockRejectedValue({ code: "error", message: "staging folder /roms/_testkopien/.Neues Platin.g4-new is in the way; remove it or choose another name", asset: null });
    await click(button("Anlegen"));
    await flush();
    expect(container.querySelector('[role="alert"]')?.textContent).toBe("Ein alter Zwischenordner liegt im Weg. Lösche ihn oder wähle einen anderen Namen.");
    await click(button("Zurück"));
    expect(current()).toBe("3. Name und Ziel");
    expect(container.querySelector('[role="alert"]')).toBeNull();
  });

  it("cancels", async () => {
    await show();
    await click(button("Abbrechen"));
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
