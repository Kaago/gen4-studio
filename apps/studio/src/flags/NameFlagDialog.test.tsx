// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nameFlag } from "../api";
import type { NameReport } from "../api";
import { pressKey, typeInto } from "../ui/primitives/testUtils";
import { NameFlagDialog, namedText } from "./NameFlagDialog";

vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), nameFlag: vi.fn() }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const report = (over: Partial<NameReport> = {}): NameReport => ({
  kind: "flag",
  number: 0x96,
  id: "FLAG_HAT_MEDAILLE",
  created: true,
  files: ["data/ids/flags.json", "data/flags/0150.json"],
  change: 4,
  ms: 3,
  warnings: [],
  ...over,
});

describe("NameFlagDialog", () => {
  let container: HTMLElement;
  let root: Root;
  const onClose = vi.fn();
  const onDone = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onClose.mockReset();
    onDone.mockReset();
    vi.mocked(nameFlag).mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  type Props = Parameters<typeof NameFlagDialog>[0];
  const show = (over: Partial<Props> = {}) =>
    act(async () =>
      root.render(<NameFlagDialog kind="flag" number={0x96} name="FLAG_UNUSED_0x0096" current={{ id: null, description: null }} onClose={onClose} onDone={onDone} {...over} />),
    );
  const dialog = () => document.querySelector<HTMLElement>("[data-name-flag-dialog]");
  const field = (label: string) => dialog()!.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`);
  const button = (text: string) => [...(dialog()?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((b) => b.textContent === text);

  it("names_with_prefix_and_shows_the_kernel_refusal", async () => {
    await show();
    expect(dialog()?.getAttribute("role")).toBe("dialog");
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Als eigenes Flag benennen");
    // The own name starts with the prefix of its kind; the hint keeps the pret name valid.
    const id = field("Eigener Name") as HTMLInputElement;
    expect(id.value).toBe("FLAG_");
    expect(dialog()?.textContent).toContain("A–Z, 0–9 und _; der pret-Name FLAG_UNUSED_0x0096 bleibt gültig.");
    expect(field("Beschreibung")?.tagName).toBe("TEXTAREA");
    // One primary button.
    expect(button("Abbrechen")).toBeDefined();
    expect([...dialog()!.querySelectorAll("button[data-kind=primary]")].map((b) => b.textContent)).toEqual(["Benennen"]);
    // The kernel refuses: its sentence under the field, the dialog stays.
    vi.mocked(nameFlag).mockRejectedValueOnce({ code: "taken", message: "FLAG_HAT_MEDAILLE hat schon die Nummer 0x0094.", asset: null });
    typeInto(id, "FLAG_HAT_MEDAILLE");
    await act(async () => button("Benennen")!.click());
    expect(nameFlag).toHaveBeenLastCalledWith({ kind: "flag", number: 0x96, id: "FLAG_HAT_MEDAILLE" });
    expect(dialog()?.querySelector("[role=alert]")?.textContent).toBe("FLAG_HAT_MEDAILLE hat schon die Nummer 0x0094.");
    expect(onDone).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    // Another name with a description goes in; Enter in the field names like the button.
    vi.mocked(nameFlag).mockResolvedValueOnce(report({ id: "FLAG_HAT_ORDEN" }));
    typeInto(id, "FLAG_HAT_ORDEN");
    typeInto(field("Beschreibung")!, "Der Orden ist da.");
    await act(async () => void pressKey(id, "Enter"));
    expect(nameFlag).toHaveBeenLastCalledWith({ kind: "flag", number: 0x96, id: "FLAG_HAT_ORDEN", description: "Der Orden ist da." });
    expect(onDone).toHaveBeenCalledWith(report({ id: "FLAG_HAT_ORDEN" }));
    expect(onClose).toHaveBeenCalled();
    expect(namedText(report({ id: "FLAG_HAT_ORDEN" }))).toBe("FLAG_HAT_ORDEN benannt (ein Schritt im Verlauf).");
    // A variable: its own title and prefix; without a pret name the hint names none.
    await act(async () => root.render(<></>));
    await show({ kind: "var", number: 0x4030, name: null });
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Als eigene Variable benennen");
    expect((field("Eigener Name") as HTMLInputElement).value).toBe("VAR_");
    expect(dialog()?.textContent).toContain("A–Z, 0–9 und _.");
  });

  it("description_only_saves", async () => {
    await show({ current: { id: "FLAG_HAT_MEDAILLE", description: "Alt." } });
    expect(dialog()?.querySelector("h2")?.textContent).toBe("Beschreibung");
    // No id field: only the description, filled with the current one.
    expect(field("Eigener Name")).toBeNull();
    const text = field("Beschreibung") as HTMLTextAreaElement;
    expect(text.value).toBe("Alt.");
    expect(button("Benennen")).toBeUndefined();
    vi.mocked(nameFlag).mockResolvedValueOnce(report({ created: false }));
    typeInto(text, "Die Medaille ist abgeholt.");
    await act(async () => button("Speichern")!.click());
    expect(nameFlag).toHaveBeenLastCalledWith({ kind: "flag", number: 0x96, description: "Die Medaille ist abgeholt." });
    expect(onDone).toHaveBeenCalledWith(report({ created: false }));
    expect(namedText(report({ created: false }))).toBe("Beschreibung gespeichert.");
    // An emptied description is sent empty: the kernel removes it.
    await act(async () => root.render(<></>));
    await show({ current: { id: "FLAG_HAT_MEDAILLE", description: "Alt." } });
    vi.mocked(nameFlag).mockResolvedValueOnce(report({ created: false }));
    typeInto(field("Beschreibung") as HTMLTextAreaElement, "  ");
    await act(async () => button("Speichern")!.click());
    expect(nameFlag).toHaveBeenLastCalledWith({ kind: "flag", number: 0x96, description: "" });
  });
});
