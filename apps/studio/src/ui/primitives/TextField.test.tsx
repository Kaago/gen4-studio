// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TextField } from "./TextField";
import { pressKey, typeInto } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TextField", () => {
  let container: HTMLElement;
  let root: Root;
  const commits = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    commits.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  function Owner(props: { multiline?: boolean; error?: string; hint?: string; type?: "text" | "search" }) {
    const [value, setValue] = useState("Route 201");
    return (
      <TextField
        label="Name"
        value={value}
        onCommit={(t) => {
          commits(t);
          setValue(t);
        }}
        {...props}
      />
    );
  }
  const field = () => container.querySelector<HTMLInputElement | HTMLTextAreaElement>("input, textarea")!;

  it("is_named_by_its_label_and_marked_as_a_draft_field", () => {
    act(() => root.render(<Owner />));
    expect(field().getAttribute("aria-label")).toBe("Name");
    expect(field().hasAttribute("data-draft")).toBe(true);
    expect(field().value).toBe("Route 201");
  });

  it("shows_the_visible_label_only_on_request", () => {
    act(() => root.render(<TextField label="Name" value="x" onCommit={() => {}} />));
    expect(container.querySelector("label")).toBeNull();
    act(() => root.render(<TextField label="Name" value="x" onCommit={() => {}} showLabel />));
    expect(container.querySelector("label")?.textContent).toBe("Name");
  });

  it("commits_on_enter_and_not_per_keystroke", () => {
    act(() => root.render(<Owner />));
    act(() => field().focus());
    typeInto(field(), "Route 2");
    typeInto(field(), "Route 202");
    expect(commits).not.toHaveBeenCalled();
    pressKey(field(), "Enter");
    expect(commits.mock.calls).toEqual([["Route 202"]]);
    expect(field().value).toBe("Route 202");
  });

  it("commits_on_blur", () => {
    act(() => root.render(<Owner />));
    act(() => field().focus());
    typeInto(field(), "Route 203");
    act(() => field().blur());
    expect(commits.mock.calls).toEqual([["Route 203"]]);
  });

  it("does_not_commit_an_unchanged_value", () => {
    act(() => root.render(<Owner />));
    act(() => field().focus());
    act(() => field().blur());
    expect(commits).not.toHaveBeenCalled();
  });

  it("takes_the_typed_text_back_on_escape_and_uses_the_key", () => {
    act(() => root.render(<Owner />));
    act(() => field().focus());
    typeInto(field(), "Route 999");
    const esc = pressKey(field(), "Escape");
    expect(field().value).toBe("Route 201");
    expect(esc.defaultPrevented).toBe(true);
    act(() => field().blur());
    expect(commits).not.toHaveBeenCalled();
  });

  it("leaves_escape_alone_when_nothing_was_typed", () => {
    act(() => root.render(<Owner />));
    expect(pressKey(field(), "Escape").defaultPrevented).toBe(false);
  });

  it("shows_the_error_and_the_hint_as_text_and_flags_the_field_invalid", () => {
    act(() => root.render(<Owner error="Der Name ist zu lang. Kürze ihn auf 12 Zeichen." hint="Höchstens 12 Zeichen" />));
    expect(container.textContent).toContain("Der Name ist zu lang. Kürze ihn auf 12 Zeichen.");
    expect(container.textContent).toContain("Höchstens 12 Zeichen");
    expect(field().getAttribute("aria-invalid")).toBe("true");
    expect(field().getAttribute("aria-describedby")).toBeTruthy();
  });

  it("disables_the_field", () => {
    act(() => root.render(<TextField label="Name" value="x" onCommit={() => {}} disabled />));
    expect(field().disabled).toBe(true);
  });

  it("renders_a_search_input_and_a_multiline_textarea", () => {
    act(() => root.render(<Owner type="search" />));
    expect((field() as HTMLInputElement).type).toBe("search");
    act(() => root.render(<Owner multiline />));
    expect(field().tagName).toBe("TEXTAREA");
  });

  it("keeps_enter_as_a_line_break_in_a_multiline_field_and_commits_on_mod_enter", () => {
    act(() => root.render(<Owner multiline />));
    act(() => field().focus());
    typeInto(field(), "a\nb");
    expect(pressKey(field(), "Enter").defaultPrevented).toBe(false);
    expect(commits).not.toHaveBeenCalled();
    pressKey(field(), "Enter", { metaKey: true });
    expect(commits.mock.calls).toEqual([["a\nb"]]);
  });

  it("commits_typed_text_when_it_unmounts_without_a_blur", () => {
    function Shell() {
      const [shown, setShown] = useState(true);
      return (
        <div>
          {shown && <TextField label="Name" value="a" onCommit={commits} />}
          <button type="button" onClick={() => setShown(false)}>
            Weiter
          </button>
        </div>
      );
    }
    act(() => root.render(<Shell />));
    typeInto(field(), "b");
    act(() => container.querySelector("button")!.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(commits.mock.calls).toEqual([["b"]]);
  });

  it("tells_a_live_listener_about_every_keystroke_and_still_commits_only_on_enter_or_blur", () => {
    const changes = vi.fn();
    function Search() {
      const [value, setValue] = useState("");
      return (
        <TextField
          label="Suchen"
          type="search"
          value={value}
          onChange={(t) => {
            changes(t);
            setValue(t);
          }}
          onCommit={commits}
        />
      );
    }
    act(() => root.render(<Search />));
    act(() => field().focus());
    typeInto(field(), "Ro");
    typeInto(field(), "Route");
    expect(changes.mock.calls).toEqual([["Ro"], ["Route"]]);
    expect(field().value).toBe("Route");
    // The owner already holds the text, so leaving the field commits nothing more.
    act(() => field().blur());
    expect(commits).not.toHaveBeenCalled();
  });

  it("does_not_call_a_live_listener_for_a_field_that_has_none", () => {
    act(() => root.render(<Owner />));
    typeInto(field(), "Route 5");
    expect(commits).not.toHaveBeenCalled();
  });
});
