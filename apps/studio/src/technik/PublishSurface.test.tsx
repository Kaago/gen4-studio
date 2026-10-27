// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { assignBase, baseStatus, checkBase, createPatch, registerBaseRom } from "../api";
import type { BaseStatus } from "../api";
import { openPath } from "../platform/dialogs";
import { pressKey } from "../ui/primitives/testUtils";
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

const baseApi = (status: BaseStatus, busy: BaseApi["busy"]): BaseApi => ({
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
    expect(button("Patch erstellen…", patchCard()!)?.getAttribute("aria-disabled")).toBe("true");

    await act(async () => finish(patchDto()));
    const done = patchCard()!.querySelector<HTMLElement>("[role=status]")!;
    expect(done.textContent).toContain("Patch erstellt: 4,7 KiB");
    expect(done.textContent).toContain("/p/Sinnoh-Remix_DSPRE_contents/build/Sinnoh-Remix-CPUD.xdelta");
    expect(done.textContent).toContain("Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.");
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(false);
    expect(button("Patch erstellen…", patchCard()!)?.hasAttribute("aria-disabled")).toBe(false);
  });

  it("a_patch_without_changes_is_a_warning_not_a_success", async () => {
    // The built ROM equals the base ROM: the patch is valid, but it changes nothing.
    vi.mocked(createPatch).mockResolvedValue(patchDto({ size: 545, target_sha1: SHA1 }));
    await show(found());
    await click(button("Patch erstellen…", patchCard()!));
    await click(button("Patch erstellen", dialog()!));
    await act(async () => {});
    const notice = patchCard()!.querySelector<HTMLElement>("[role=status]")!;
    expect(notice.textContent).toContain("Patch ohne Änderungen: 545 Byte");
    expect(notice.textContent).toContain("/p/Sinnoh-Remix_DSPRE_contents/build/Sinnoh-Remix-CPUD.xdelta");
    expect(notice.textContent).toContain("Die gebaute ROM gleicht der Basis-ROM; der Patch ändert nichts.");
    expect(notice.textContent).not.toContain("Patch erstellt");
    expect(notice.textContent).not.toContain("Selbstprüfung bestanden");
    expect(notice.className).toContain("warning");
    // A patch with changes stays a success.
    vi.mocked(createPatch).mockResolvedValue(patchDto());
    await click(button("Patch erstellen…", patchCard()!));
    await click(button("Patch erstellen", dialog()!));
    await act(async () => {});
    const done = patchCard()!.querySelector<HTMLElement>("[role=status]")!;
    expect(done.textContent).toContain("Patch erstellt: 4,7 KiB");
    expect(done.className).toContain("success");
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
        for (const [status, busy] of [
      [{ state: "unassigned", suggestion: SIBLING }, "patch"],
      [{ state: "unassigned", suggestion: null }, "patch"],
      [missing(), "patch"],
      [{ state: "unassigned", suggestion: SIBLING }, "check"],
      [missing(), "register"],
    ] as [BaseStatus, BaseApi["busy"]][]) {
      await act(async () => root.render(<PublishSurface base={baseApi(status, busy)} building={false} onOpenProject={onOpenProject} />));
      const buttons = [...baseCard()!.querySelectorAll("button")];
      expect(buttons.length).toBeGreaterThan(0);
      // Busy blocks with aria-disabled, so the focused button keeps focus; „Patch erstellen…“ without a found base stays `disabled`.
      for (const b of buttons) expect(b.getAttribute("aria-disabled")).toBe("true");
      expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(true);
    }
    await act(async () => root.render(<PublishSurface base={baseApi(found(), "patch")} building={false} onOpenProject={onOpenProject} />));
    expect(button("Patch erstellen…", patchCard()!)?.getAttribute("aria-disabled")).toBe("true");
    expect(patchCard()?.textContent).toContain("Baue und erstelle den Patch …");
  });

  it("an_app_build_disables_the_patch_and_the_base_actions", async () => {
    // A build or save of the app runs (session busy): nothing here may start another build or change the project.
    await show(found());
    await act(async () => root.render(<Host building />));
    expect(button("Patch erstellen…", patchCard()!)?.getAttribute("aria-disabled")).toBe("true");
    expect(patchCard()?.textContent).not.toContain("Baue und erstelle den Patch …");
    await act(async () => root.render(<Host />));
    expect(button("Patch erstellen…", patchCard()!)?.disabled).toBe(false);
    expect(button("Patch erstellen…", patchCard()!)?.hasAttribute("aria-disabled")).toBe(false);

    act(() => root.unmount());
    root = createRoot(container);
    vi.mocked(baseStatus).mockResolvedValue({ state: "unassigned", suggestion: SIBLING });
    await act(async () => root.render(<Host building />));
    await act(async () => {});
    for (const b of baseCard()!.querySelectorAll("button")) expect(b.getAttribute("aria-disabled")).toBe("true");
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

  describe("keyboard focus", () => {
    const settle = () => act(async () => {});
    /** Focuses a button and presses it, as Enter does. */
    const press = async (el: HTMLButtonElement | undefined) => {
      el!.focus();
      await click(el);
      await settle();
    };
    const focused = () => document.activeElement;
    // The native file dialog took focus to the body meanwhile.
    const dialogTakesFocus = (path: string | null) =>
      vi.mocked(openPath).mockImplementationOnce(async () => {
        (document.activeElement as HTMLElement).blur();
        return path;
      });

    it("a_running_check_keeps_focus_on_the_button_and_a_second_activation_does_nothing", async () => {
      let answer: (c: ReturnType<typeof check>) => void = () => {};
      vi.mocked(checkBase).mockReturnValue(new Promise((resolve) => (answer = resolve)));
      await show();
      const assign = button("Zuordnen…", baseCard()!) as HTMLButtonElement;
      const other = button("Andere Datei wählen…", baseCard()!) as HTMLButtonElement;
      assign.focus();
      await click(assign);
      // Busy blocks with aria-disabled, not `disabled`: the focused button keeps focus.
      expect(focused()).toBe(assign);
      expect(assign.disabled).toBe(false);
      expect(assign.getAttribute("aria-disabled")).toBe("true");
      expect(other.getAttribute("aria-disabled")).toBe("true");
      await click(assign);
      await click(other);
      expect(checkBase).toHaveBeenCalledOnce();
      expect(openPath).not.toHaveBeenCalled();
      // „Patch erstellen…“ is blocked for another reason (no base ROM): it stays natively disabled.
      expect((button("Patch erstellen…", patchCard()!) as HTMLButtonElement).disabled).toBe(true);
      await act(async () => answer(check()));
      expect(dialogTitle()).toBe("Basis-ROM zuordnen?");
      expect(assign.hasAttribute("aria-disabled")).toBe(false);
    });

    it("closing_the_assign_dialog_without_assigning_returns_focus_to_the_button_that_opened_it", async () => {
      const opened = async (name: string) => {
        await press(button(name, baseCard()!));
        expect(dialogTitle()).toBe("Basis-ROM zuordnen?");
        // Focus is inside the dialog while it is open (jsdom does not always move it there by itself).
        dialog()!.querySelector("button")!.focus();
        expect(dialog()!.contains(focused())).toBe(true);
      };
      const back = async (name: string) => {
        await settle();
        expect(dialog()).toBeNull();
        expect(focused()).toBe(button(name, baseCard()!));
      };
      await show();
      // Abbrechen
      await opened("Zuordnen…");
      await click(button("Abbrechen", dialog()!));
      await back("Zuordnen…");
      // Escape
      await opened("Zuordnen…");
      await act(async () => void pressKey(focused() as HTMLElement, "Escape"));
      await back("Zuordnen…");
      // The cross
      await opened("Zuordnen…");
      await click(dialog()!.querySelector("[aria-label=Schließen]"));
      await back("Zuordnen…");
      // „Andere Datei wählen…“ opens it after the native dialog took the focus away.
      dialogTakesFocus("/roms/other.nds");
      await opened("Andere Datei wählen…");
      await click(button("Abbrechen", dialog()!));
      await back("Andere Datei wählen…");
      // A refusal: only „Schließen“.
      vi.mocked(checkBase).mockResolvedValueOnce(check({ refused: "Falsches Spiel: die ROM ist IPKD, das Projekt CPUD." }));
      await opened("Zuordnen…");
      await click(button("Schließen", dialog()!));
      await back("Zuordnen…");
      // An assignment that fails.
      vi.mocked(assignBase).mockRejectedValueOnce({ code: "error", message: "Schreiben gescheitert.", asset: null, problems: [] });
      await opened("Zuordnen…");
      await click(button("Zuordnen", dialog()!));
      await settle();
      await back("Zuordnen…");
      expect(baseCard()?.getAttribute("data-base-state")).toBe("unassigned");
    });

    it("a_failed_or_cancelled_check_keeps_focus_on_the_button", async () => {
      await show();
      vi.mocked(checkBase).mockRejectedValueOnce({ code: "error", message: "Die ROM ist nicht lesbar.", asset: null, problems: [] });
      await press(button("Zuordnen…", baseCard()!));
      expect(dialog()).toBeNull();
      expect(focused()).toBe(button("Zuordnen…", baseCard()!));
      expect(baseCard()?.querySelector("[role=alert]")?.textContent).toBe("Die ROM ist nicht lesbar.");
      // Cancelled file dialog, and one that took the focus: back on the button.
      dialogTakesFocus(null);
      await press(button("Andere Datei wählen…", baseCard()!));
      expect(focused()).toBe(button("Andere Datei wählen…", baseCard()!));
    });

    it("after_a_successful_assignment_focus_goes_to_patch_erstellen", async () => {
      await show();
      await press(button("Zuordnen…", baseCard()!));
      await click(button("Zuordnen", dialog()!));
      await settle();
      expect(dialog()).toBeNull();
      expect(button("Zuordnen…")).toBeUndefined();
      const patch = button("Patch erstellen…", patchCard()!) as HTMLButtonElement;
      expect(patch.disabled).toBe(false);
      expect(focused()).toBe(patch);
    });

    it("basis_rom_suchen_keeps_focus_while_it_runs_and_moves_to_patch_erstellen_on_success", async () => {
      await show(missing());
      let finish: (s: BaseStatus) => void = () => {};
      vi.mocked(registerBaseRom).mockReturnValueOnce(new Promise((resolve) => (finish = resolve)));
      vi.mocked(openPath).mockResolvedValue("/roms/renamed.nds");
      const search = button("Basis-ROM suchen…", baseCard()!) as HTMLButtonElement;
      search.focus();
      await click(search);
      await settle();
      expect(focused()).toBe(search);
      expect(search.disabled).toBe(false);
      expect(search.getAttribute("aria-disabled")).toBe("true");
      await click(search);
      expect(registerBaseRom).toHaveBeenCalledOnce();
      await act(async () => finish(found({ path: "/roms/renamed.nds" })));
      await settle();
      expect(baseCard()?.getAttribute("data-base-state")).toBe("found");
      expect(button("Basis-ROM suchen…")).toBeUndefined();
      expect(focused()).toBe(button("Patch erstellen…", patchCard()!));
    });

    it("a_failed_or_cancelled_search_keeps_focus_on_basis_rom_suchen", async () => {
      await show(missing());
      vi.mocked(openPath).mockResolvedValue("/roms/HeartGold.nds");
      vi.mocked(registerBaseRom).mockRejectedValueOnce({ code: "error", message: "Das ist eine andere ROM.", asset: null, problems: [] });
      await press(button("Basis-ROM suchen…", baseCard()!));
      expect(baseCard()?.querySelector("[role=alert]")?.textContent).toBe("Das ist eine andere ROM.");
      expect(focused()).toBe(button("Basis-ROM suchen…", baseCard()!));
      dialogTakesFocus(null);
      await press(button("Basis-ROM suchen…", baseCard()!));
      expect(registerBaseRom).toHaveBeenCalledOnce();
      expect(focused()).toBe(button("Basis-ROM suchen…", baseCard()!));
    });

    it("patch_erstellen_keeps_focus_when_cancelled_while_running_and_after_the_patch", async () => {
      await show(found());
      const patch = () => button("Patch erstellen…", patchCard()!) as HTMLButtonElement;
      await press(patch());
      expect(dialogTitle()).toBe("Patch erstellen?");
      await click(button("Abbrechen", dialog()!));
      await settle();
      expect(focused()).toBe(patch());

      let finish: (dto: ReturnType<typeof patchDto>) => void = () => {};
      vi.mocked(createPatch).mockReturnValue(new Promise((resolve) => (finish = resolve)));
      await press(patch());
      await click(button("Patch erstellen", dialog()!));
      await settle();
      // The patch builds: the button is aria-disabled and has focus, a second activation does nothing.
      expect(focused()).toBe(patch());
      expect(patch().disabled).toBe(false);
      expect(patch().getAttribute("aria-disabled")).toBe("true");
      await click(patch());
      expect(dialog()).toBeNull();
      await act(async () => finish(patchDto()));
      await settle();
      expect(patchCard()?.textContent).toContain("Patch erstellt: 4,7 KiB");
      expect(focused()).toBe(patch());
      expect(patch().hasAttribute("aria-disabled")).toBe(false);
    });

    it("focus_the_user_moved_elsewhere_while_a_step_ran_stays_there", async () => {
      await show({ state: "unassigned", suggestion: null });
      const elsewhere = document.createElement("button");
      container.append(elsewhere);
      vi.mocked(openPath).mockImplementationOnce(async () => {
        elsewhere.focus();
        return null;
      });
      await press(button("Basis-ROM zuordnen…", baseCard()!));
      expect(focused()).toBe(elsewhere);
    });
  });

  it("the_patch_note_describes_the_patch_button", async () => {
    await show();
    const describedBy = () => button("Patch erstellen…", patchCard()!)?.getAttribute("aria-describedby");
    // Blocked: the reason.
    expect(document.getElementById(describedBy()!)?.textContent).toBe("Erst eine Basis-ROM zuordnen.");
    // Found and idle: nothing to describe.
    await act(async () => root.render(<PublishSurface base={baseApi(found(), null)} building={false} onOpenProject={onOpenProject} />));
    expect(button("Patch erstellen…", patchCard()!)?.hasAttribute("aria-describedby")).toBe(false);
    // Running: the note about the build.
    await act(async () => root.render(<PublishSurface base={baseApi(found(), "patch")} building={false} onOpenProject={onOpenProject} />));
    expect(document.getElementById(describedBy()!)?.textContent).toBe("Baue und erstelle den Patch …");
  });
});
