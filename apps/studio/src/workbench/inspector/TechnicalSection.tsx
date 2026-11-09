import { useTranslation } from "react-i18next";
import { Section } from "../../ui/primitives";
import type { InspectorModel } from "../Inspector";
import { useSectionOpen } from "./foldState";
import styles from "./TechnicalSection.module.css";
import { assetLine, filePath, technicalRows } from "./technical";
import type { TechnicalSource } from "./technical";

/**
 * The section "Technisch" of the depth Tüfteln: the asset line, the file path and every field by its raw key with its decimal and
 * hex value. Closed until opened. Editors render it only in Tüfteln, once their record is loaded.
 */
export function TechnicalSection({ model, source }: { model: InspectorModel; source: TechnicalSource }) {
  const { t } = useTranslation("workbench");
  const [open, toggle] = useSectionOpen("technisch", false);
  const path = filePath(model);
  return (
    <Section title={t("technicalSection.title")} open={open} onToggle={toggle}>
      <dl className={styles.facts}>
        <dt>{t("technicalSection.asset")}</dt>
        <dd>{assetLine(model)}</dd>
        {path !== null && (
          <>
            <dt>{t("technicalSection.file")}</dt>
            <dd>{path}</dd>
          </>
        )}
      </dl>
      <table className={styles.table}>
        <thead>
          <tr>
            <th scope="col">{t("technicalSection.field")}</th>
            <th scope="col">{t("technicalSection.dec")}</th>
            <th scope="col">{t("technicalSection.hex")}</th>
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
