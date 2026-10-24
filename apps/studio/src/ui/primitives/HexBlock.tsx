import styles from "./HexBlock.module.css";

/** Where a field lies in a member's bytes. Structurally the `FieldSpanDto` of `data_raw`. */
export type HexFieldSpan = { name: string; offset: number; len: number; kind: string };

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

/** The value of a field of up to four bytes, little endian as the DS stores it; longer fields (bit sets) have none. */
function valueOf(bytes: readonly number[], field: HexFieldSpan): number | undefined {
  if (field.len < 1 || field.len > 4) return undefined;
  let value = 0;
  for (let i = field.len - 1; i >= 0; i--) value = value * 256 + (bytes[field.offset + i] ?? 0);
  return value;
}

function footerOf(bytes: readonly number[], field: HexFieldSpan, label: string): string {
  const last = field.offset + field.len - 1;
  const where = field.len === 1 ? `Byte ${field.offset}` : `Byte ${field.offset} bis ${last}`;
  const value = valueOf(bytes, field);
  return value === undefined ? `${where}: ${label}` : `${where}: ${label}, 0x${hex(value, field.len * 2)} = ${value}`;
}

/** The raw bytes of a member, 16 to a row with their offsets; the highlighted field is marked and named below. */
export function HexBlock({ bytes, fields, highlight, path, member, labelOf = (name) => name }: HexBlockProps) {
  const field = highlight === undefined ? undefined : fields.find((f) => f.name === highlight);
  const marked = (offset: number) => field !== undefined && offset >= field.offset && offset < field.offset + field.len;
  const rows = Array.from({ length: Math.ceil(bytes.length / PER_ROW) }, (_, r) => r * PER_ROW);
  return (
    <figure className={styles.block}>
      <figcaption className={styles.caption}>
        <span className={styles.path}>{path}</span>, Eintrag {member}, {bytes.length} {bytes.length === 1 ? "Byte" : "Bytes"}
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
      {field && <p className={styles.footer}>{footerOf(bytes, field, labelOf(field.name))}</p>}
    </figure>
  );
}
