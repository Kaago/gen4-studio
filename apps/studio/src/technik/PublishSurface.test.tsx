// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { assignBase, baseStatus, checkBase, createPatch, registerBaseRom } from "../api";
import type { BaseStatus } from "../api";
import { openPath } from "../platform/dialogs";
import { SHA1, check, found, missing, patchDto } from "./baseTesting";
import { PublishSurface } from "./PublishSurface";
import { useBase } from "./useBase";
import type { BaseApi } from "./useBase";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  baseStatus: vi.fn(),
  checkBase: vi.fn(),
  assignBase: vi.fn(),
  registerBaseRom: vi.fn(),
  createPatch: vi.fn(),
}));
vi.mock("../platform/dialogs", async (importOriginal) => ({ ...(await importOriginal<typeof import("../platform/dialogs")>()), openPath: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const PROJECT = "/p/Sinnoh-Remix_DSPRE_contents";
const SIBLING = "/p/Sinnoh-Remix.nds";

describe("PublishSurface", () => {
  let container: HTMLElement;
  let root: Root;
  const onOpenProject = vi.fn();

  function Host({ building = false }: { building?: boolean }) {
    const base = useBase({ project: PROJECT, active: true });
    return <PublishSurface base={base} building={building} onOpenProject={onOpenProject} />;
  }

  beforeEach(() => {
    onOpenProject.mockClear();
    vi.mocked(baseStatus).mockReset().mockResolvedValue({ state: "unassigned", suggestion: SIBLING });
    vi.mocked(checkBase).mockReset().mockResolvedValue(check());
    vi.mocked(assignBase).mockReset().mockResolvedValue(found({ path: SIBLING, source: "sibling" }));
    vi.mocked(registerBaseRom).mockReset();
    vi.mocked(createPatch).mockReset().mockResolvedValue(patchDto());
    vi.mocked(openPath).mockReset().mockResolvedValue(null);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const show = async (status?: BaseStatus) => {
    if (status) vi.mocked(baseStatus).mockResolvedValue(status);
    await act(async () => root.render(<Host />));
    await act(async () => {});
  };
  const surface = () => container.querySelector<HTMLElement>("[data-technik=veroeffentlichen]");
  const baseCard = () => container.querySelector<HTMLElement>("[data-base-state]");
  const patchCard = () => container.querySelector<HTMLElement>("[data-patch-card]");
  const button = (name: string, scope: ParentNode = container) => [...scope.querySelectorAll("button")].find((b) => b.textContent === name);
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  const dialogTitle = () => document.getElementById(dialog()?.getAttribute("aria-labelledby") ?? "")?.textContent;
  const click = (el: Element | null | undefined) => act(async () => void (el as HTMLElement).click());

  it("without_a_project_offers_to_open_one", async () => {
    await act(async () => root.render(<PublishSurface base={null} building={false} onOpenProject={onOpenProject} />));
    expect(surface()?.querySelector("h1")?.textContent).toBe("Veröffentlichen");
    expect(surface()?.textContent).toContain("Kein Projekt geöffnet.");
    expect(surface()?.textContent).toContain("Öffne ein Projekt, dann erstellt Technik hier seinen Patch.");
    expect(baseCard()).toBeNull();
    await click(button("Projekt öffnen"));
    expect(onOpenProject).toHaveBeenCalledOnce();
    expect(baseStatus).not.toHaveBeenCalled();
  });

  it("unassigned_with_a_suggestion_offers_zuordnen", async () => {
    await show();
    const card = baseCard()!;
    expect(card.getAttribute("data-base-state")).toBe("unassigned");
    expect(card.querySelector("h2")?.textContent).toBe("Basis-ROM");
    expect(card.textContent).toContain("Ein Patch beschreibt den Unterschied zu deiner Basis-ROM. Die ROM bleibt außerhalb des Projekts; Gen4 Studio merkt sich nur, wo sie liegt.");
    expect(card.textContent).toContain("Nicht zugeordnet");
    expect(card.querySelector("[role=status]")?.textContent).toContain("Neben dem Projekt liegt „Sinnoh-Remix.nds“.");
    expect(button("Zuordnen…", card)?.getAttribute("data-kind")).toBe("primary");
    expect(button("Andere Datei wählen…", card)?.getAttribute("data-kind")).toBe("secondary");
    // The patch waits for a base.
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(true);
    expect(patchCard()?.textContent).toContain("Erst eine Basis-ROM zuordnen.");
    expect(patchCard()?.textContent).toContain("Der Patch enthält nur deine Änderungen. Wer dieselbe Basis-ROM hat, wendet ihn mit xdelta3 oder Rom Patcher JS an.");

    // „Andere Datei wählen…“ asks for a .nds and checks it.
    vi.mocked(openPath).mockResolvedValue("/roms/other.nds");
    await click(button("Andere Datei wählen…", card));
    expect(openPath).toHaveBeenCalledWith({ directory: false, title: "Basis-ROM wählen", filters: [{ name: "DS-ROM", extensions: ["nds"] }] });
    expect(checkBase).toHaveBeenCalledWith("/roms/other.nds");
    expect(dialogTitle()).toBe("Basis-ROM zuordnen?");
  });

  it("unassigned_without_a_suggestion_asks_for_a_file", async () => {
    await show({ state: "unassigned", suggestion: null });
    const card = baseCard()!;
    expect(card.textContent).toContain("Diesem Projekt ist keine Basis-ROM zugeordnet.");
    expect(button("Zuordnen…", card)).toBeUndefined();
    expect(button("Basis-ROM zuordnen…", card)?.getAttribute("data-kind")).toBe("primary");
    // Cancelled: nothing is checked.
    await click(button("Basis-ROM zuordnen…", card));
    expect(openPath).toHaveBeenCalledOnce();
    expect(checkBase).not.toHaveBeenCalled();
    expect(dialog()).toBeNull();
  });

  it("the_check_dialog_lists_the_warnings_and_assigns_on_confirm", async () => {
    vi.mocked(checkBase).mockResolvedValue(
      check({ warnings: [{ kind: "title_differs", message: "Der Titel der ROM („POKEMON PL“) weicht vom Projekt ab („POKEMON PT“)." }, { kind: "unknown_dump", message: "Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei." }] }),
    );
    await show();
    await click(button("Zuordnen…", baseCard()!));
    expect(checkBase).toHaveBeenCalledWith(SIBLING);
    const asked = dialog()!;
    expect(dialogTitle()).toBe("Basis-ROM zuordnen?");
    expect(asked.textContent).toContain(`POKEMON PL · CPUD · SHA-1 ${SHA1.slice(0, 8)}`);
    expect(asked.textContent).toContain("Bekannte Basis-ROM: Platin DE (Testprojekt)");
    const warnings = [...asked.querySelectorAll("[role=status]")].map((n) => n.textContent);
    expect(warnings).toEqual([
      "Der Titel der ROM („POKEMON PL“) weicht vom Projekt ab („POKEMON PT“).",
      "Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.",
    ]);
    expect(asked.textContent).toContain("Schreibt die Basismarke .g4/base.json ins Projekt.");
    expect(asked.textContent).toContain("Nicht rückgängig");
    expect(button("Abbrechen", asked)).toBeDefined();

    await click(button("Abbrechen", asked));
    expect(dialog()).toBeNull();
    expect(assignBase).not.toHaveBeenCalled();

    await click(button("Zuordnen…", baseCard()!));
    expect(button("Zuordnen", dialog()!)?.getAttribute("data-kind")).toBe("primary");
    await click(button("Zuordnen", dialog()!));
    await act(async () => {});
    expect(assignBase).toHaveBeenCalledWith(SIBLING);
    expect(dialog()).toBeNull();
    expect(baseCard()?.getAttribute("data-base-state")).toBe("found");
    expect(baseCard()?.textContent).toContain("Basis-ROM zugeordnet: Platin DE (Testprojekt).");
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(false);
  });

  it("the_check_dialog_keeps_its_content_while_it_assigns", async () => {
    let finish: (status: BaseStatus) => void = () => {};
    vi.mocked(assignBase).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    vi.mocked(checkBase).mockResolvedValue(check({ warnings: [{ kind: "unknown_dump", message: "Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei." }] }));
    await show();
    await click(button("Zuordnen…", baseCard()!));
    await click(button("Zuordnen", dialog()!));
    expect(assignBase).toHaveBeenCalledWith(SIBLING);
    // The assignment runs: the dialog still says what it assigns, and nothing in it starts a second one.
    const asked = dialog()!;
    expect(asked.hasAttribute("data-closed")).toBe(false);
    expect(dialogTitle()).toBe("Basis-ROM zuordnen?");
    expect(asked.textContent).toContain(`POKEMON PL · CPUD · SHA-1 ${SHA1.slice(0, 8)}`);
    expect(asked.textContent).toContain("Bekannte Basis-ROM: Platin DE (Testprojekt)");
    expect(asked.querySelector("[role=status]")?.textContent).toBe("Unbekannte oder veränderte ROM; Patches gelten nur für genau diese Datei.");
    expect(asked.textContent).toContain("Schreibt die Basismarke .g4/base.json ins Projekt.");
    expect(button("Zuordnen", asked)?.disabled).toBe(true);
    expect(button("Abbrechen", asked)?.disabled).toBe(true);

    await act(async () => finish(found({ path: SIBLING, source: "sibling" })));
    await act(async () => {});
    expect(dialog()).toBeNull();
    expect(baseCard()?.textContent).toContain("Basis-ROM zugeordnet: Platin DE (Testprojekt).");
  });

  it("a_wrong_game_shows_the_reason_and_only_schliessen", async () => {
    const refused = "Falsches Spiel: die ROM ist IPKD, das Projekt CPUD.";
    vi.mocked(checkBase).mockResolvedValue(check({ info: { ...check().info, game_code: "IPKD", known_base: null }, refused }));
    await show();
    await click(button("Zuordnen…", baseCard()!));
    const asked = dialog()!;
    expect(asked.querySelector("[role=alert]")?.textContent).toBe(refused);
    expect(asked.textContent).not.toContain("Bekannte Basis-ROM");
    expect(asked.textContent).not.toContain("Schreibt die Basismarke");
    expect([...asked.querySelectorAll("button")].filter((b) => b.textContent !== "").map((b) => b.textContent)).toEqual(["Schließen"]);
    await click(button("Schließen", asked));
    expect(dialog()).toBeNull();
    expect(assignBase).not.toHaveBeenCalled();
  });

  it("missing_base_disables_the_patch_and_offers_basis_rom_suchen", async () => {
    await show(missing());
    const card = baseCard()!;
    expect(card.getAttribute("data-base-state")).toBe("missing");
    expect(card.textContent).toContain("Fehlt");
    expect(card.querySelector("[role=status]")?.textContent).toContain("Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).");
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(true);
    expect(patchCard()?.textContent).toContain("Basis-ROM nicht gefunden: Platin DE (Testprojekt) (d40f19ac…).");

    // „Basis-ROM suchen…“ remembers where it lies now; a refusal says why and keeps the state.
    vi.mocked(openPath).mockResolvedValue("/roms/HeartGold.nds");
    vi.mocked(registerBaseRom).mockRejectedValueOnce({ code: "error", message: "Das ist eine andere ROM (SHA-1 13b6853a…), gesucht ist Platin DE (Testprojekt) (d40f19ac…).", asset: null, problems: [] });
    await click(button("Basis-ROM suchen…", card));
    await act(async () => {});
    expect(openPath).toHaveBeenCalledWith({ directory: false, title: "Basis-ROM wählen", filters: [{ name: "DS-ROM", extensions: ["nds"] }] });
    expect(registerBaseRom).toHaveBeenCalledWith("/roms/HeartGold.nds");
    expect(baseCard()?.getAttribute("data-base-state")).toBe("missing");
    expect(baseCard()?.querySelector("[role=alert]")?.textContent).toContain("Das ist eine andere ROM (SHA-1 13b6853a…), gesucht ist Platin DE (Testprojekt) (d40f19ac…).");

    vi.mocked(openPath).mockResolvedValue("/roms/renamed.nds");
    vi.mocked(registerBaseRom).mockResolvedValue(found({ path: "/roms/renamed.nds" }));
    await click(button("Basis-ROM suchen…", baseCard()!));
    await act(async () => {});
    expect(baseCard()?.getAttribute("data-base-state")).toBe("found");
    expect(baseCard()?.querySelector("[role=alert]")).toBeNull();
  });

  it("unreadable_mark_shows_the_reason", async () => {
    const reason = "Die Basismarke .g4/base.json ist nicht lesbar (expected value); entferne sie, dann ordne neu zu.";
    await show({ state: "unreadable", reason });
    expect(baseCard()?.getAttribute("data-base-state")).toBe("unreadable");
    expect(baseCard()?.textContent).toContain("Nicht lesbar");
    expect(baseCard()?.querySelector("[role=alert]")?.textContent).toBe(reason);
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(true);
    expect(patchCard()?.textContent).toContain(reason);
  });

  it("found_base_shows_label_short_sha1_and_path", async () => {
    await show(found());
    const card = baseCard()!;
    expect(card.getAttribute("data-base-state")).toBe("found");
    expect(card.textContent).toContain("Gefunden");
    const fact = (term: string) => [...card.querySelectorAll("dt")].find((dt) => dt.textContent === term)?.nextElementSibling as HTMLElement | undefined;
    expect(fact("ROM")?.textContent).toBe("Platin DE (Testprojekt)");
    expect(fact("SHA-1")?.textContent).toBe("d40f19ac");
    expect(fact("SHA-1")?.getAttribute("title")).toBe(SHA1);
    expect(fact("Datei")?.textContent).toBe("/Users/me/Roms/Pokemon - Platin-Edition (Germany).nds");
    expect(fact("Datei")?.getAttribute("title")).toBe("/Users/me/Roms/Pokemon - Platin-Edition (Germany).nds");
    // Nothing to assign any more.
    expect(button("Zuordnen…")).toBeUndefined();
    expect(button("Basis-ROM suchen…")).toBeUndefined();
    expect(patchCard()?.textContent).not.toContain("Erst eine Basis-ROM zuordnen.");
  });

  it("create_patch_asks_first_then_shows_size_path_and_self_check", async () => {
    // The name comes from the backend's status (the kernel's rule), not from the folder.
    await show(found({ patch_name: "Mein_Hack-CPUD" }));
    await click(button("Patch erstellen…", patchCard()!));
    expect(createPatch).not.toHaveBeenCalled();
    const asked = dialog()!;
    expect(dialogTitle()).toBe("Patch erstellen?");
    expect(asked.textContent).toContain(
      "Speichert, baut die ROM und schreibt build/Mein_Hack-CPUD.xdelta und build/Mein_Hack-CPUD.xdelta.txt. Ein älterer Patch dort wird ersetzt.",
    );
    await click(button("Abbrechen", asked));
    expect(dialog()).toBeNull();
    expect(createPatch).not.toHaveBeenCalled();

    let finish: (dto: ReturnType<typeof patchDto>) => void = () => {};
    vi.mocked(createPatch).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    await click(button("Patch erstellen…", patchCard()!));
    expect(button("Patch erstellen", dialog()!)?.getAttribute("data-kind")).toBe("primary");
    await click(button("Patch erstellen", dialog()!));
    expect(createPatch).toHaveBeenCalledOnce();
    expect(dialog()).toBeNull();
    // While it runs: the note, and nothing else changes the project.
    expect(patchCard()?.textContent).toContain("Baue und erstelle den Patch …");
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(true);

    await act(async () => finish(patchDto()));
    const done = patchCard()!.querySelector<HTMLElement>("[role=status]")!;
    expect(done.textContent).toContain("Patch erstellt: 4,7 KiB");
    expect(done.textContent).toContain("/p/Sinnoh-Remix_DSPRE_contents/build/Sinnoh-Remix-CPUD.xdelta");
    expect(done.textContent).toContain("Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.");
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(false);
  });

  it("a_failed_patch_shows_the_reason_in_the_patch_card", async () => {
    vi.mocked(createPatch).mockRejectedValue({ code: "error", message: "Die Selbstprüfung ist gescheitert; kein Patch geschrieben.", asset: null, problems: [] });
    await show(found());
    await click(button("Patch erstellen…", patchCard()!));
    await click(button("Patch erstellen", dialog()!));
    await act(async () => {});
    expect(patchCard()?.querySelector("[role=alert]")?.textContent).toBe("Die Selbstprüfung ist gescheitert; kein Patch geschrieben.");
    expect(baseCard()?.querySelector("[role=alert]")).toBeNull();
  });

  it("while_something_runs_no_action_changes_the_project", async () => {
    const fake = (status: BaseStatus, busy: BaseApi["busy"]): BaseApi => ({
      status,
      error: null,
      errorFrom: null,
      busy,
      lastPatch: null,
      check: vi.fn(async () => null),
      assign: vi.fn(async () => {}),
      register: vi.fn(async () => {}),
      createPatch: vi.fn(async () => {}),
      refresh: vi.fn(async () => {}),
    });
    for (const [status, busy] of [
      [{ state: "unassigned", suggestion: SIBLING }, "patch"],
      [{ state: "unassigned", suggestion: null }, "patch"],
      [missing(), "patch"],
      [{ state: "unassigned", suggestion: SIBLING }, "check"],
      [missing(), "register"],
    ] as [BaseStatus, BaseApi["busy"]][]) {
      await act(async () => root.render(<PublishSurface base={fake(status, busy)} building={false} onOpenProject={onOpenProject} />));
      const buttons = [...baseCard()!.querySelectorAll("button")];
      expect(buttons.length).toBeGreaterThan(0);
      for (const b of buttons) expect(b.disabled).toBe(true);
    }
    await act(async () => root.render(<PublishSurface base={fake(found(), "patch")} building={false} onOpenProject={onOpenProject} />));
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(true);
    expect(patchCard()?.textContent).toContain("Baue und erstelle den Patch …");
  });

  it("an_app_build_disables_the_patch_and_the_base_actions", async () => {
    // A build or save of the app runs (session busy): nothing here may start another build or change the project.
    await show(found());
    await act(async () => root.render(<Host building />));
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(true);
    expect(patchCard()?.textContent).not.toContain("Baue und erstelle den Patch …");
    await act(async () => root.render(<Host />));
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(false);

    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(baseStatus).mockResolvedValue({ state: "unassigned", suggestion: SIBLING });
    await act(async () => root.render(<Host building />));
    await act(async () => {});
    for (const b of baseCard()!.querySelectorAll("button")) expect(b.disabled).toBe(true);
  });

  it("the_assigned_notice_goes_when_the_next_step_starts", async () => {
    let finish: (dto: ReturnType<typeof patchDto>) => void = () => {};
    vi.mocked(createPatch).mockReturnValue(new Promise((resolve) => (finish = resolve)));
    await show();
    await click(button("Zuordnen…", baseCard()!));
    await click(button("Zuordnen", dialog()!));
    await act(async () => {});
    expect(baseCard()?.textContent).toContain("Basis-ROM zugeordnet: Platin DE (Testprojekt).");
    await click(button("Patch erstellen…", patchCard()!));
    await click(button("Patch erstellen", dialog()!));
    expect(baseCard()?.textContent).not.toContain("Basis-ROM zugeordnet");
    await act(async () => finish(patchDto()));
    expect(baseCard()?.textContent).not.toContain("Basis-ROM zugeordnet");
  });
});
