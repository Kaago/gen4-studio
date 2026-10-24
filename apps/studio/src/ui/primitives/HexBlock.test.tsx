// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { HexBlock } from "./HexBlock";
import type { HexFieldSpan } from "./HexBlock";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// An item entry: price 300 (0x012C, little endian), then a few more bytes; 36 bytes in all.
const bytes = [0x2c, 0x01, 0x05, 0x00, 0x1e, ...Array.from({ length: 31 }, (_, i) => i)];
const fields: HexFieldSpan[] = [
  { name: "price", offset: 0, len: 2, kind: "u16" },
  { name: "hold_effect", offset: 2, len: 1, kind: "u8" },
  { name: "fling_power", offset: 4, len: 1, kind: "u8" },
];
const labels: Record<string, string> = { price: "Preis", hold_effect: "Trageeffekt", fling_power: "Schleuderstärke" };

describe("HexBlock", () => {
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

  const byteCells = () => [...container.querySelectorAll<HTMLElement>("td[data-raw]")];

  it("lays_the_bytes_out_sixteen_to_a_row_with_offsets", () => {
    act(() => root.render(<HexBlock bytes={bytes} fields={fields} path="itemtool/itemdata/pl_item_data.narc" member={17} />));
    expect(container.querySelector("table")).not.toBeNull();
    const rows = [...container.querySelectorAll("tbody tr")];
    expect(rows).toHaveLength(3);
    expect(rows.map((r) => r.querySelector("th")?.textContent)).toEqual(["0000", "0010", "0020"]);
    expect(rows.map((r) => r.querySelectorAll("td[data-raw]").length)).toEqual([16, 16, 4]);
    expect(byteCells()).toHaveLength(36);
    expect(byteCells()[0]!.textContent).toBe("2C");
    expect(byteCells()[0]!.getAttribute("data-raw")).toBe("2C");
    expect(byteCells()[0]!.getAttribute("data-offset")).toBe("0");
    expect(container.textContent).toContain("itemtool/itemdata/pl_item_data.narc");
    expect(container.textContent).toContain("Eintrag 17, 36 Bytes");
  });

  it("marks_only_the_bytes_of_the_highlighted_field_and_names_it_in_the_footer", () => {
    act(() =>
      root.render(
        <HexBlock bytes={bytes} fields={fields} highlight="price" path="pl_item_data.narc" member={17} labelOf={(name) => labels[name] ?? name} />,
      ),
    );
    const marked = byteCells().filter((c) => c.hasAttribute("data-marked"));
    expect(marked.map((c) => c.getAttribute("data-offset"))).toEqual(["0", "1"]);
    expect(container.textContent).toContain("Byte 0 bis 1: Preis, 0x012C = 300");
  });

  it("a_one_byte_field_says_a_single_byte", () => {
    act(() => root.render(<HexBlock bytes={bytes} fields={fields} highlight="fling_power" path="pl_item_data.narc" member={17} labelOf={(name) => labels[name] ?? name} />));
    expect(byteCells().filter((c) => c.hasAttribute("data-marked")).map((c) => c.getAttribute("data-offset"))).toEqual(["4"]);
    expect(container.textContent).toContain("Byte 4: Schleuderstärke, 0x1E = 30");
  });

  it("reads_an_i8_field_signed", () => {
    // A move entry: priority -1 at byte 5.
    const move = [0, 0, 0, 0, 0, 0xff, 0, 0];
    const spans: HexFieldSpan[] = [{ name: "priority", offset: 5, len: 1, kind: "i8" }];
    act(() => root.render(<HexBlock bytes={move} fields={spans} highlight="priority" path="pl_waza_tbl.narc" member={17} labelOf={() => "Priorität"} />));
    expect(container.textContent).toContain("Byte 5: Priorität, 0xFF = −1");
  });

  it("reads_a_bits_field_under_its_mask_and_names_the_bit_range", () => {
    // Bytes 8 and 9 hold 0x0D80 (little endian): under mask 0x0780 that is 0x0580, bits 7 to 10 read 0b1011 = 11; bit 11 is set.
    const item = [0, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x0d, 0, 0];
    const spans: HexFieldSpan[] = [
      { name: "field_pocket", offset: 8, len: 2, kind: "bits", mask: 0x0780 },
      { name: "registrable", offset: 8, len: 2, kind: "bits", mask: 0x0800 },
    ];
    act(() => root.render(<HexBlock bytes={item} fields={spans} highlight="field_pocket" path="pl_item_data.narc" member={17} labelOf={() => "Feldtasche"} />));
    expect(container.textContent).toContain("Byte 8 bis 9, Bits 7 bis 10: Feldtasche, 11");
    expect(byteCells().filter((c) => c.hasAttribute("data-marked")).map((c) => c.getAttribute("data-offset"))).toEqual(["8", "9"]);
    act(() => root.render(<HexBlock bytes={item} fields={spans} highlight="registrable" path="pl_item_data.narc" member={17} labelOf={() => "Registrierbar"} />));
    expect(container.textContent).toContain("Byte 8 bis 9, Bit 11: Registrierbar, 1");
  });

  it("names_a_bytes_field_without_a_value", () => {
    const personal = Array.from({ length: 44 }, () => 0xff);
    const spans: HexFieldSpan[] = [{ name: "tm", offset: 28, len: 13, kind: "bytes" }];
    act(() => root.render(<HexBlock bytes={personal} fields={spans} highlight="tm" path="pl_personal.narc" member={397} labelOf={() => "TM und VM"} />));
    expect(container.querySelector("p")?.textContent).toBe("Byte 28 bis 40: TM und VM");
  });

  it("marks_nothing_without_a_known_highlight", () => {
    act(() => root.render(<HexBlock bytes={bytes} fields={fields} highlight="nope" path="pl_item_data.narc" member={17} />));
    expect(byteCells().filter((c) => c.hasAttribute("data-marked"))).toHaveLength(0);
    expect(container.textContent).not.toContain("Byte 0");
  });
});
