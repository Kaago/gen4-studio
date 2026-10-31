// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PublishOptionsDto, PublishPhase, PublishReportDto, ReleasesDto } from "../api";
import { openPath } from "../platform/dialogs";
import { PublishWizard } from "./PublishWizard";
import type { PublishWizardProps } from "./PublishWizard";
import { publishReportOf, releaseOf, releasesOf } from "./publishTesting";
import { BAD_FILE, BAD_VERSION, NO_ROM_CONTENT, VERSION_TAKEN } from "./publishWords";

vi.mock("../platform/dialogs", async (importOriginal) => ({ ...(await importOriginal<typeof import("../platform/dialogs")>()), openPath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
const EXISTS = "Sinnoh-Remix-v0.1.1.xdelta liegt schon am Speicherort; wähle einen anderen Namen oder ersetze die Datei.";

describe("PublishWizard", () => {
  let container: HTMLElement;
  let root: Root;
  const onPublish = vi.fn<PublishWizardProps["onPublish"]>();
  const onReveal = vi.fn<PublishWizardProps["onReveal"]>();
  const onCommit = vi.fn<PublishWizardProps["onCommit"]>();
  const onClose = vi.fn<PublishWizardProps["onClose"]>();
  /** One release published already: the proposal is 0.1.1. */
  const LISTED: ReleasesDto = releasesOf({ next_version: "0.1.1", releases: [releaseOf()] });

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    for (const fn of [onReveal, onCommit, onClose]) fn.mockReset();
    onReveal.mockResolvedValue(undefined);
    onPublish.mockReset().mockImplementation(async (opts) => publishReportOf(opts));
    vi.mocked(openPath).mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  async function show(over: Partial<PublishWizardProps> = {}) {
    const props: PublishWizardProps = { defaults: LISTED, defaultsError: null, gitOwn: true, onPublish, onReveal, onCommit, onClose, ...over };
    await act(async () => root.render(<PublishWizard {...props} />));
    await flush();
  }
  const text = () => container.textContent ?? "";
  const button = (name: string) => [...container.querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === name);
  const click = (el: Element | null | undefined) => act(async () => void el?.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  const next = () => click(button("Weiter"));
  const field = (label: string) => container.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`)!;
  const step = () => container.querySelector("[data-publish-step]")?.getAttribute("data-publish-step");
  /** The sentence below an invalid field (the error comes last in the field), null for a valid one. */
  const fieldError = (label: string) => (field(label).getAttribute("aria-invalid") === "true" ? (field(label).parentElement?.lastElementChild?.textContent ?? "") : null);
  const box = (label: string) => [...container.querySelectorAll<HTMLElement>("[role=checkbox]")].find((b) => b.closest("label")?.textContent === label);
  async function type(input: HTMLInputElement | HTMLTextAreaElement, value: string) {
    const proto = input instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    await act(async () => input.focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => input.blur());
    await flush();
  }
  /** On to „Patch“ and through the run, to „Prüfen“. */
  async function publish() {
    await next();
    await next();
    await flush();
  }

  it("proposes_the_next_version_and_the_file_name", async () => {
    await show();
    // The four steps, the last one the result.
    expect([...container.querySelectorAll('[aria-label="Schritte"] li')].map((li) => li.getAttribute("aria-label"))).toEqual(["1. Version", "2. Patch", "3. Prüfen", "4. Fertig"]);
    expect(step()).toBe("version");
    expect(field("Version").value).toBe("0.1.1");
    expect(field("Dateiname").value).toBe("Sinnoh-Remix-v0.1.1.xdelta");
    expect(container.querySelector("[data-publish-dir]")?.textContent).toBe("/h/Sinnoh-Remix/build");
    expect(text()).toContain("Was sich seit der letzten Fassung geändert hat. Steht in der Begleitdatei.");
    expect(text()).toContain("Schreibt Sinnoh-Remix-v0.1.1.xdelta und die Begleitdatei nach /h/Sinnoh-Remix/build.");
    expect(button("Weiter")?.disabled).toBe(false);

    // While the list is read nothing can go on; a list that cannot be read says why and locks.
    await show({ defaults: null });
    expect(button("Weiter")?.disabled).toBe(true);
    await show({ defaults: null, defaultsError: "Die Liste der Veröffentlichungen ist nicht lesbar: kaputt" });
    expect(container.querySelector("[role=alert]")?.textContent).toBe("Die Liste der Veröffentlichungen ist nicht lesbar: kaputt");
    expect(button("Weiter")?.disabled).toBe(true);
  });

  it("a_taken_or_unsafe_version_locks_weiter_with_the_reason", async () => {
    await show();
    await type(field("Version"), "0.1.0");
    expect(fieldError("Version")).toBe(VERSION_TAKEN);
    expect(button("Weiter")?.disabled).toBe(true);
    await type(field("Version"), "0.2 beta");
    expect(fieldError("Version")).toBe(BAD_VERSION);
    expect(button("Weiter")?.disabled).toBe(true);
    await type(field("Version"), "1/0");
    expect(fieldError("Version")).toBe(BAD_VERSION);
    // Enter does not move on either.
    await act(async () => void field("Dateiname").dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
    expect(step()).toBe("version");
    await type(field("Version"), "0.2.0");
    expect(fieldError("Version")).toBeNull();
    expect(button("Weiter")?.disabled).toBe(false);
    // A file name a file cannot have locks too.
    await type(field("Dateiname"), "a:b");
    expect(fieldError("Dateiname")).toBe(BAD_FILE);
    expect(button("Weiter")?.disabled).toBe(true);
    expect(onPublish).not.toHaveBeenCalled();
  });

  it("the_file_name_follows_the_version_until_edited", async () => {
    await show();
    await type(field("Version"), "1.0.0");
    expect(field("Dateiname").value).toBe("Sinnoh-Remix-v1.0.0.xdelta");
    await type(field("Dateiname"), "Remix-Final");
    await type(field("Version"), "1.0.1");
    expect(field("Dateiname").value).toBe("Remix-Final");
    // The summary names the file with its suffix, as the kernel writes it.
    expect(text()).toContain("Schreibt Remix-Final.xdelta und die Begleitdatei nach /h/Sinnoh-Remix/build.");
  });

  it("choosing_a_folder_updates_the_summary", async () => {
    await show();
    vi.mocked(openPath).mockResolvedValue(null);
    await click(button("Ordner wählen…"));
    expect(openPath).toHaveBeenCalledWith({ directory: true, title: "Speicherort wählen" });
    expect(container.querySelector("[data-publish-dir]")?.textContent).toBe("/h/Sinnoh-Remix/build");
    vi.mocked(openPath).mockResolvedValue("/h/Weitergabe");
    await click(button("Ordner wählen…"));
    expect(container.querySelector("[data-publish-dir]")?.textContent).toBe("/h/Weitergabe");
    expect(text()).toContain("Schreibt Sinnoh-Remix-v0.1.1.xdelta und die Begleitdatei nach /h/Weitergabe.");
    await publish();
    expect(onPublish.mock.calls[0]![0]).toMatchObject({ dir: "/h/Weitergabe" });
  });

  it("weiter_on_patch_runs_publish_with_the_options_and_shows_the_phases", async () => {
    let progress: (p: { phase: PublishPhase }) => void = () => {};
    let finish: (r: PublishReportDto) => void = () => {};
    onPublish.mockImplementation((_opts, onProgress) => {
      progress = onProgress;
      return new Promise((resolve) => (finish = resolve));
    });
    await show();
    await type(field("Änderungsnotizen"), "Neue Route\nZweite Zeile");
    await next();
    expect(step()).toBe("patch");
    expect(text()).toContain("Speichert, baut die ROM und schreibt den Patch gegen die Basis-ROM. Die gebaute ROM bleibt unter build/ im Projekt.");
    const readme = box("Änderungsnotizen auch als README.txt daneben")!;
    expect(readme.getAttribute("aria-checked")).toBe("false");
    await click(readme);
    expect(readme.getAttribute("aria-checked")).toBe("true");
    expect(onPublish).not.toHaveBeenCalled();

    await next();
    const opts: PublishOptionsDto = { version: "0.1.1", notes: "Neue Route\nZweite Zeile", dir: "/h/Sinnoh-Remix/build", file: "Sinnoh-Remix-v0.1.1.xdelta", readme: true };
    expect(onPublish).toHaveBeenCalledOnce();
    expect(onPublish.mock.calls[0]![0]).toEqual(opts);
    // Before the first phase arrives (and in the browser mode, where none comes) the first line shows.
    const status = () => container.querySelector('[role="status"][data-publish-progress]')?.textContent;
    expect(status()).toBe("Speichern und bauen …");
    expect(button("Zurück")?.disabled).toBe(true);
    for (const [phase, line] of [
      ["encode", "Patch kodieren …"],
      ["verify", "Selbstprüfung …"],
      ["write", "Dateien schreiben …"],
      ["record", "Veröffentlichung eintragen …"],
    ] as const) {
      await act(async () => progress({ phase }));
      expect(status()).toBe(line);
    }
    await act(async () => finish(publishReportOf(opts)));
    await flush();
    expect(step()).toBe("check");
    expect(onPublish).toHaveBeenCalledOnce();
  });

  it("an_existing_file_offers_ersetzen", async () => {
    onPublish.mockRejectedValueOnce({ code: "exists", message: EXISTS, asset: null });
    await show();
    await publish();
    // Refused before the build: the sentence, „Ersetzen“ beside it, the wizard stays on „Patch“.
    expect(step()).toBe("patch");
    expect(container.querySelector("[role=alert]")?.textContent).toBe(EXISTS);
    const replace = container.querySelector<HTMLButtonElement>("[data-publish-overwrite]");
    expect(replace?.textContent).toBe("Ersetzen");
    expect(replace?.getAttribute("data-kind")).toBe("secondary");
    await click(replace);
    await flush();
    expect(onPublish).toHaveBeenCalledTimes(2);
    expect(onPublish.mock.calls[1]![0]).toMatchObject({ version: "0.1.1", overwrite: true });
    expect(onPublish.mock.calls[0]![0].overwrite).toBeUndefined();
    expect(step()).toBe("check");

    // Another refusal has no „Ersetzen“.
    onPublish.mockReset().mockRejectedValueOnce({ code: "no_changes", message: "Die gebaute ROM gleicht der Basis-ROM; es gibt nichts zu veröffentlichen.", asset: null });
    await act(async () => root.render(null));
    await show();
    await publish();
    expect(container.querySelector("[role=alert]")?.textContent).toBe("Die gebaute ROM gleicht der Basis-ROM; es gibt nichts zu veröffentlichen.");
    expect(container.querySelector("[data-publish-overwrite]")).toBeNull();

    // A version the backend refuses (published meanwhile) goes back to „Version“ with the sentence at the field.
    onPublish.mockReset().mockRejectedValueOnce({ code: "version_taken", message: VERSION_TAKEN, asset: null });
    await act(async () => root.render(null));
    await show();
    await publish();
    expect(step()).toBe("version");
    expect(fieldError("Version")).toBe(VERSION_TAKEN);
    expect(button("Weiter")?.disabled).toBe(true);
    await type(field("Version"), "0.1.2");
    expect(fieldError("Version")).toBeNull();
  });

  it("pruefen_shows_the_self_check_and_the_files", async () => {
    await show();
    await next();
    await click(box("Änderungsnotizen auch als README.txt daneben"));
    await next();
    await flush();
    expect(step()).toBe("check");
    const ok = [...container.querySelectorAll('[role="status"]')].find((n) => n.textContent?.includes("Selbstprüfung bestanden"));
    expect(ok?.textContent).toContain("Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.");
    expect(ok?.textContent).toContain("Basis d40f19ac → Ziel 7e3a1b9c, geprüft in 148 ms");
    const preview = container.querySelector("[data-publish-preview]")!;
    expect(preview.textContent).toContain("Was du weitergibst");
    const rows = [...preview.querySelectorAll("tbody tr, [role=row]")].map((r) => [...r.querySelectorAll("td, [role=cell], [role=gridcell]")].map((c) => c.textContent));
    expect(rows.filter((r) => r.length > 0)).toEqual([
      ["Sinnoh-Remix-v0.1.1.xdelta", "4,4 KiB"],
      ["Sinnoh-Remix-v0.1.1.xdelta.txt", "600 Byte"],
      ["README.txt", "122 Byte"],
    ]);
    expect(preview.textContent).toContain(NO_ROM_CONTENT);
    expect(button("Fertig")).toBeDefined();
    expect(button("Weiter")).toBeUndefined();
    // Back from here runs nothing again: the release is written.
    await click(button("Zurück"));
    expect(step()).toBe("check");
    expect(onPublish).toHaveBeenCalledOnce();
  });

  it("fertig_shows_the_result_reveals_and_offers_stand_sichern_only_with_git", async () => {
    await show();
    await type(field("Änderungsnotizen"), "Neue Route");
    await publish();
    await click(button("Fertig"));
    expect(text()).toContain("Veröffentlicht: v0.1.1");
    expect(text()).toContain("/h/Sinnoh-Remix/build/Sinnoh-Remix-v0.1.1.xdelta");
    expect(text()).toContain("2 Dateien, 5,0 KiB");
    expect(text()).toContain("Den Patch weitergeben: wer dieselbe Basis-ROM hat, wendet ihn mit Rom Patcher JS oder xdelta3 an.");
    expect(text()).toContain("Stand sichern legt einen Commit mit „Veröffentlicht v0.1.1“ an.");
    await click(button("Im Finder zeigen"));
    expect(onReveal).toHaveBeenCalledWith("/h/Sinnoh-Remix/build/Sinnoh-Remix-v0.1.1.xdelta");
    const commit = container.querySelector<HTMLButtonElement>("[data-publish-commit]");
    expect(commit?.textContent).toBe("Stand sichern…");
    expect(commit?.getAttribute("data-kind")).toBe("secondary");
    await click(commit);
    expect(onCommit).toHaveBeenCalledWith({ message: "Veröffentlicht v0.1.1", notes: "Neue Route" });
    await click(button("Schließen"));
    expect(onClose).toHaveBeenCalledWith(true);

    // Without the project's own repository: no „Stand sichern…“ and no line about it.
    await act(async () => root.render(null));
    await show({ gitOwn: false });
    await publish();
    await click(button("Fertig"));
    expect(text()).toContain("Veröffentlicht: v0.1.1");
    expect(container.querySelector("[data-publish-commit]")).toBeNull();
    expect(text()).not.toContain("Stand sichern");
  });

  it("leaving_before_the_run_says_nothing_was_published", async () => {
    await show();
    await click(button("Abbrechen"));
    expect(onClose).toHaveBeenCalledWith(false);
  });
});
