// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../cssSource";
import { ChangeTable } from "./ChangeTable";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ChangeTable", () => {
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

  const cells = (row: Element | undefined) => [...(row?.querySelectorAll("th, td") ?? [])];

  it("renders_columns_rows_and_marks_spans_with_del_and_ins", () => {
    act(() =>
      root.render(
        <ChangeTable
          columns={["Feld", "Commit", "Projekt"]}
          ariaLabel="Trainer 0001"
          rows={[
            { key: "a", field: "data.party[0].level", old: "5", new: "6" },
            {
              key: "b",
              field: "Wetter",
              title: "weather_id",
              old: [
                { op: "same", text: "Hallo " },
                { op: "removed", text: "Welt" },
              ],
              new: [
                { op: "same", text: "Hallo " },
                { op: "added", text: "Sinnoh" },
              ],
            },
          ]}
          more="und 3 weitere Felder"
        />,
      ),
    );
    const table = container.querySelector("table")!;
    // A real table, named by a caption that is read but not shown.
    expect(table.querySelector("caption")?.textContent).toBe("Trainer 0001");
    expect(table.querySelector("caption")?.className).toMatch(/hidden/);
    expect([...table.querySelectorAll("thead th")].map((th) => th.textContent)).toEqual(["Feld", "Commit", "Projekt"]);
    expect([...table.querySelectorAll("thead th")].every((th) => th.getAttribute("scope") === "col")).toBe(true);
    const rows = [...table.querySelectorAll("tbody tr")];
    expect(rows).toHaveLength(2);
    // The field is the row's header, in code; the values are code too.
    const first = cells(rows[0]);
    expect(first[0]?.tagName).toBe("TH");
    expect(first[0]?.getAttribute("scope")).toBe("row");
    expect(first.map((c) => c.textContent)).toEqual(["data.party[0].level", "5", "6"]);
    expect(first.every((c) => c.querySelector("code") !== null)).toBe(true);
    // The title of a field is its tooltip (the JSON path behind a German name).
    expect(cells(rows[1])[0]?.querySelector("[title]")?.getAttribute("title")).toBe("weather_id");
    // Spans: removed words in <del>, added ones in <ins>, the rest plain.
    const second = cells(rows[1]);
    expect(second[1]?.querySelector("del")?.textContent).toBe("Welt");
    expect(second[1]?.querySelector("ins")).toBeNull();
    expect(second[2]?.querySelector("ins")?.textContent).toBe("Sinnoh");
    expect(second[1]?.textContent).toBe("Hallo Welt");
    // More rows than shown: a line below the table.
    expect(container.textContent).toContain("und 3 weitere Felder");
    // Not colour alone: struck through and underlined on the status surfaces.
    const css = readCss("./ChangeTable.module.css", import.meta.url);
    expect(ruleBody(css, ".removed")).toMatch(/background:\s*var\(--g4-status-fehler-flaeche\)/);
    expect(ruleBody(css, ".removed")).toMatch(/text-decoration[^;]*line-through/);
    expect(ruleBody(css, ".added")).toMatch(/background:\s*var\(--g4-status-erfolg-flaeche\)/);
    expect(ruleBody(css, ".added")).toMatch(/text-decoration[^;]*underline/);
    expect(second[1]?.querySelector("del")?.className).toMatch(/removed/);
    expect(second[2]?.querySelector("ins")?.className).toMatch(/added/);
  });

  it("absent_reads_as_dash_with_fehlt", () => {
    act(() => root.render(<ChangeTable columns={["Feld", "Basis", "Projekt"]} ariaLabel="Item 0004" rows={[{ key: "a", field: "data.price", old: null, new: "200" }]} />));
    const [, old, now] = cells(container.querySelector("tbody tr")!);
    // „—“ shown, „fehlt“ read.
    expect(old?.textContent).toBe("—fehlt");
    expect(old?.querySelector("[aria-hidden=true]")?.textContent).toBe("—");
    expect(old?.querySelector("[class*=hidden]")?.textContent).toBe("fehlt");
    expect(now?.textContent).toBe("200");
    // No footer without `more`.
    expect(container.querySelector("[data-change-table-more]")).toBeNull();
  });

  it("breaks_a_path_only_at_dots_and_brackets", () => {
    act(() =>
      root.render(
        <ChangeTable
          columns={["Feld", "Commit", "Projekt"]}
          ariaLabel="Trainer 0001"
          rows={[
            { key: "a", field: "data.party[0].level", old: "5", new: "6" },
            { key: "b", field: <span>Wetter</span>, old: "1", new: "2" },
          ]}
        />,
      ),
    );
    const [first, second] = [...container.querySelectorAll("tbody th code")];
    // A soft break after each dot and before each bracket, nowhere else; the text stays the path.
    expect(first?.innerHTML).toBe("data.<wbr>party<wbr>[0].<wbr>level");
    expect(first?.textContent).toBe("data.party[0].level");
    // A field that is no string is shown as given.
    expect(second?.innerHTML).toBe("<span>Wetter</span>");
    // Edges: a leading bracket or dot, brackets in a row, a dot before a bracket (one break), dots in a row, a trailing dot.
    const shownAs = (field: string) => {
      act(() => root.render(<ChangeTable columns={["Feld", "Commit", "Projekt"]} ariaLabel="x" rows={[{ key: "a", field, old: "1", new: "2" }]} />));
      return container.querySelector("tbody th code")?.innerHTML;
    };
    expect(shownAs("[0].a")).toBe("[0].<wbr>a");
    expect(shownAs(".a")).toBe(".<wbr>a");
    expect(shownAs("x[1][2].y")).toBe("x<wbr>[1]<wbr>[2].<wbr>y");
    expect(shownAs("a.[0]")).toBe("a.<wbr>[0]");
    expect(shownAs("a..b")).toBe("a.<wbr>.<wbr>b");
    expect(shownAs("a.")).toBe("a.");
    expect(shownAs("Größe")).toBe("Größe");
    // No lookbehind: WebKit before 16.4 (macOS 13.0–13.2, which the app supports) cannot parse one.
    expect(readCss("./ChangeTable.tsx", import.meta.url)).not.toMatch(/\(\?<[=!]/);
  });

  it("sizes_the_field_column_to_its_content_and_lets_the_sides_share_the_rest", () => {
    const css = readCss("./ChangeTable.module.css", import.meta.url);
    // Auto layout: a fixed layout gives every column a third whatever it holds.
    expect(ruleBody(css, ".table")).not.toMatch(/table-layout:\s*fixed/);
    // The field column as narrow as allowed: its path at its own width up to a limit, broken only where a <wbr> allows (or a segment
    // longer than the limit), never mid-word as `anywhere` would.
    expect(ruleBody(css, ".table tbody th")).toMatch(/width:\s*1%/);
    expect(ruleBody(css, ".table tbody th")).not.toMatch(/overflow-wrap:\s*anywhere/);
    const path = ruleBody(css, ".path");
    expect(path).toMatch(/width:\s*max-content/);
    // The limit: so many characters, at most a share of the table (its wrapper is the container), so a narrow detail keeps room for the sides.
    expect(ruleBody(css, ".wrap")).toMatch(/container-type:\s*inline-size/);
    expect(path).toMatch(/max-width:\s*min\(\d+ch,\s*\d+cqi\)/);
    expect(path).toMatch(/word-break:\s*normal/);
    expect(path).toMatch(/overflow-wrap:\s*break-word/);
    // A long value (a SHA-1) still wraps inside its side instead of widening the table.
    expect(ruleBody(css, ".table td")).toMatch(/overflow-wrap:\s*anywhere/);
  });
});
