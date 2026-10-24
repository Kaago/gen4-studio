// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { Badge } from "./Badge";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Badge", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("uses_the_default_text_of_its_kind_and_an_icon", () => {
    act(() => root.render(<Badge kind="irreversible" />));
    expect(container.textContent).toBe("Nicht rückgängig");
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("prefers_children_over_the_default_text", () => {
    act(() => root.render(<Badge kind="info">Neu</Badge>));
    expect(container.textContent).toBe("Neu");
  });

  it("takes_a_symbol_for_a_kind_without_one", () => {
    act(() => root.render(<Badge kind="neutral">Platin, DE</Badge>));
    expect(container.querySelector("svg")).toBeNull();
    act(() => root.render(<Badge kind="neutral" icon="game">Platin, DE</Badge>));
    expect(container.querySelector("svg")).not.toBeNull();
    expect(container.textContent).toBe("Platin, DE");
  });

  it("has_a_default_text_for_every_kind", () => {
    const expected = {
      unsaved: "Ungespeichert",
      irreversible: "Nicht rückgängig",
      locked: "Gesperrt",
      changedOnDisk: "Auf Platte geändert",
      ptHgssOnly: "Nur Platin und HGSS",
      error: "Fehler",
      live: "Live",
      info: "Info",
      success: "Erledigt",
      warning: "Warnung",
      code: "Code",
      neutral: "Hinweis",
    } as const;
    for (const [kind, text] of Object.entries(expected)) {
      act(() => root.render(<Badge kind={kind as keyof typeof expected} />));
      expect(container.textContent).toBe(text);
    }
  });

  it("renders_the_code_kind_as_plain_text_with_a_title_and_without_a_symbol", () => {
    act(() => root.render(<Badge kind="code" title={"\\x0123"}>{"{COLOR, 0, 5}"}</Badge>));
    expect(container.textContent).toBe("{COLOR, 0, 5}");
    expect(container.querySelector("svg")).toBeNull();
    expect(container.querySelector("[title]")?.getAttribute("title")).toBe("\\x0123");
  });

  it("shows_a_symbol_for_success_and_warning", () => {
    for (const kind of ["success", "warning"] as const) {
      act(() => root.render(<Badge kind={kind} />));
      expect(container.querySelector("svg")).not.toBeNull();
    }
  });
});
