import { useState } from "react";
import { Section } from "../../ui/primitives";
import type { InspectorModel } from "../Inspector";
import styles from "./TechnicalSection.module.css";
import { assetLine, filePath, technicalRows } from "./technical";
import type { TechnicalSource } from "./technical";

/**
 * The section "Technisch" of the depth Tüfteln: the asset line, the file path and every field by its raw key with its decimal and
 * hex value. Closed until opened. Editors render it only in Tüfteln, once their record is loaded.
 */
export function TechnicalSection({ model, source }: { model: InspectorModel; source: TechnicalSource }) {
  const [open, setOpen] = useState(false);
  const path = filePath(model);
  return (
    <Section title="Technisch" open={open} onToggle={() => setOpen((o) => !o)}>
      <dl className={styles.facts}>
        <dt>Asset</dt>
        <dd>{assetLine(model)}</dd>
        {path !== null && (
          <>
            <dt>Datei</dt>
            <dd>{path}</dd>
          </>
        )}
      </dl>
      <table className={styles.table}>
        <thead>
          <tr>
            <th scope="col">Feld</th>
            <th scope="col">Dez</th>
            <th scope="col">Hex</th>
          </tr>
        </thead>
        <tbody>
          {technicalRows(source).map((row) => (
            <tr key={row.key}>
              <th scope="row">{row.key}</th>
              <td>{row.dec}</td>
              <td>{row.hex ?? "–"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Section>
  );
}
