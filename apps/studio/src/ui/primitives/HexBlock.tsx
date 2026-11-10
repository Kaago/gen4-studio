import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import styles from "./HexBlock.module.css";

/** How a field's bytes read: unsigned or signed little-endian numbers, a bit range under `mask`, or plain bytes. */
export type HexFieldKind = "u8" | "u16" | "u32" | "i8" | "bits" | "bytes";

/** Where a field lies in a member's bytes. Structurally the `FieldSpanDto` of `data_raw` (`mask` only for `bits`). */
export type HexFieldSpan = { name: string; offset: number; len: number; kind: HexFieldKind; mask?: number };

export type HexBlockProps = {
  bytes: readonly number[];
  fields: readonly HexFieldSpan[];
  /** The `name` of the field to mark (the one last focused in the form). */
  highlight?: string;
  /** The archive the bytes come from, shown in mono. */
  path: string;
  member: number;
  /** The German name of a field for the footer ("price" → "Preis"); without, the field's own name. */
  labelOf?: (name: string) => string;
};

const PER_ROW = 16;
const hex = (value: number, digits: number) => value.toString(16).toUpperCase().padStart(digits, "0");

/** The unsigned little-endian number in a field of one to four bytes (the DS stores little endian). */
function rawOf(bytes: readonly number[], field: HexFieldSpan): number | undefined {
  if (field.len < 1 || field.len > 4) return undefined;
  let value = 0;
  for (let i = field.len - 1; i >= 0; i--) value = value * 256 + (bytes[field.offset + i] ?? 0);
  return value;
}

/** The lowest and highest set bit of a mask. */
function bitRange(mask: number): [number, number] {
  const m = mask >>> 0;
  return [31 - Math.clz32(m & -m), 31 - Math.clz32(m)];
}

/** The footer line: where the field lies, its name, and its value as the field's kind reads it. */
function footerOf(bytes: readonly number[], field: HexFieldSpan, label: string, t: TFunction<"ui">): string {
  const last = field.offset + field.len - 1;
  const where = field.len === 1 ? t("hexBlock.byte", { offset: field.offset }) : t("hexBlock.bytes", { from: field.offset, to: last });
  const raw = rawOf(bytes, field);
  if (field.kind === "bits") {
    if (field.mask === undefined || field.mask === 0 || raw === undefined) return `${where}: ${label}`;
    const [low, high] = bitRange(field.mask);
    const bits = low === high ? t("hexBlock.bit", { bit: low }) : t("hexBlock.bits", { from: low, to: high });
    return `${where}, ${bits}: ${label}, ${((raw & field.mask) >>> 0) >>> low}`;
  }
  if (field.kind === "bytes" || raw === undefined) return `${where}: ${label}`;
  const shown = field.kind === "i8" && raw >= 0x80 ? raw - 0x100 : raw;
  // A negative value with a typographic minus sign.
  const value = shown < 0 ? `\u2212${-shown}` : `${shown}`;
  return `${where}: ${label}, 0x${hex(raw, field.len * 2)} = ${value}`;
}

/** The raw bytes of a member, 16 to a row with their offsets; the highlighted field is marked and named below. */
export function HexBlock({ bytes, fields, highlight, path, member, labelOf = (name) => name }: HexBlockProps) {
  const { t } = useTranslation("ui");
  const field = highlight === undefined ? undefined : fields.find((f) => f.name === highlight);
  const marked = (offset: number) => field !== undefined && offset >= field.offset && offset < field.offset + field.len;
  const rows = Array.from({ length: Math.ceil(bytes.length / PER_ROW) }, (_, r) => r * PER_ROW);
  return (
    <figure className={styles.block}>
      <figcaption className={styles.caption}>
        <span className={styles.path}>{path}</span>
        {t("hexBlock.captionRest", { member, count: bytes.length })}
      </figcaption>
      <table className={styles.table}>
        <tbody>
          {rows.map((start) => (
            <tr key={start}>
              <th scope="row" className={styles.offset}>
                {hex(start, 4)}
              </th>
              {bytes.slice(start, start + PER_ROW).map((byte, i) => {
                const offset = start + i;
                const on = marked(offset);
                return (
                  <td
                    key={offset}
                    className={on ? `${styles.byte} ${styles.marked}` : styles.byte}
                    data-raw={hex(byte, 2)}
                    data-offset={offset}
                    data-marked={on ? "" : undefined}
                  >
                    {hex(byte, 2)}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
      {field && <p className={styles.footer}>{footerOf(bytes, field, labelOf(field.name), t)}</p>}
    </figure>
  );
}
