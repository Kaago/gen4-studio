import type { MatrixDto } from "../../api";
import { Section } from "../../ui/primitives";
import { rawRow } from "../matrix";
import { useSectionOpen } from "./foldState";
import styles from "./RawRowSection.module.css";

/**
 * The section "Rohdaten, Zeile y" of the depth Tüfteln: the row of the chosen cell as it lies in the file, one line each for the maps
 * and the headers (u16, four hex digits in capitals) and the altitudes (u8). The chosen cell is marked in every line.
 */
export function RawRowSection({ matrix, x, y }: { matrix: MatrixDto; x: number; y: number }) {
  const [open, toggle] = useSectionOpen("rohdaten", true);
  const title = `Rohdaten, Zeile ${y}`;
  const row = rawRow(matrix, y);
  const lines: { name: string; values: readonly (string | number)[] }[] = [{ name: "Karte", values: row.maps }];
  if (row.headers) lines.push({ name: "Header", values: row.headers });
  if (row.altitudes) lines.push({ name: "Höhe", values: row.altitudes });
  return (
    <Section title={title} open={open} onToggle={toggle}>
      <div className={styles.scroll}>
        <table className={styles.table} aria-label={title}>
          <thead>
            <tr>
              <td />
              {row.maps.map((_, column) => (
                <th key={column} scope="col">
                  {column}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {lines.map((line) => (
              <tr key={line.name}>
                <th scope="row">{line.name}</th>
                {line.values.map((value, column) => (
                  <td key={column} aria-current={column === x ? "true" : undefined} className={column === x ? styles.chosen : undefined}>
                    {value}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Section>
  );
}
