import { useEffect, useState } from "react";
import { dataRaw, errorText, fieldLabels } from "../../api";
import type { DataKind, RawDto } from "../../api";
import { HexBlock } from "../../ui/primitives";
import type { Editing } from "./PokedexHand";
import styles from "./Pokedex.module.css";

/** The parts of a species with their raw data, in the order the tab shows them. */
const PARTS: readonly { kind: DataKind; title: string }[] = [
  { kind: "species", title: "Pokémon-Daten" },
  { kind: "learnset", title: "Lernset" },
  { kind: "evolution", title: "Entwicklungen" },
];

const EVO_FIELD = /^(method|param|target)(\d)$/;
const EVO_WORD: Record<string, string> = { method: "Methode", param: "Parameter", target: "Ziel" };

/** The German name of a field of the raw data ("attack" → "Angriff", "param0" → "Entwicklung 1 Parameter"). */
function labelOf(name: string): string {
  const evo = EVO_FIELD.exec(name);
  if (evo) return `Entwicklung ${Number(evo[2]) + 1} ${EVO_WORD[evo[1]!]}`;
  return (fieldLabels.species as Record<string, string>)[name] ?? name;
}

/** One part's bytes as the store holds them, read again with every new document (an edit, undo, redo). */
function RawPart({ e, kind, title, highlight }: { e: Editing; kind: DataKind; title: string; highlight: string | undefined }) {
  const [raw, setRaw] = useState<{ raw: RawDto | null; error: string | null } | null>(null);
  useEffect(() => {
    let current = true;
    dataRaw(kind, e.index).then(
      (r) => current && setRaw({ raw: r, error: null }),
      (err: unknown) => current && setRaw({ raw: null, error: errorText(err) }),
    );
    return () => {
      current = false;
    };
  }, [kind, e.index, e.doc]);
  return (
    <section className={styles.rawPart} aria-label={title}>
      <h3 className={styles.columnTitle}>{title}</h3>
      {raw === null ? (
        <p className={styles.note}>Lädt …</p>
      ) : raw.raw === null ? (
        <p className={styles.note}>{`Nicht lesbar: ${raw.error ?? ""}`}</p>
      ) : (
        <HexBlock bytes={raw.raw.bytes} fields={raw.raw.fields} highlight={highlight} path={raw.raw.path} member={raw.raw.member} labelOf={labelOf} />
      )}
    </section>
  );
}

/** Rohdaten (Tüfteln): the bytes of the personal data, the learnset and the evolutions; the field focused last is marked. */
export function RawTab({ e, highlight }: { e: Editing; highlight: string | undefined }) {
  return (
    <div className={styles.raw}>
      {PARTS.map((p) => (
        <RawPart key={p.kind} e={e} kind={p.kind} title={p.title} highlight={highlight} />
      ))}
    </div>
  );
}
