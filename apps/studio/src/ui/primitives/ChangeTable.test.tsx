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
});
