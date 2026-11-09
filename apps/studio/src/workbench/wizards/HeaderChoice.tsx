import { useCallback, useMemo, useState } from "react";
import type { EventFileChoice, HeaderPatchDto, HeaderSummary, MemberCounts, WarpLink } from "../../api";
import { Badge, Button, Notice, NumberField, Reference, Segmented, TextField } from "../../ui/primitives";
import type { SegmentedOption } from "../../ui/primitives";
import { headerLabel } from "../headers";
import { HeaderPicker } from "../inspector/HeaderPicker";
import { id4, termLabel, termPlural } from "../terms";
import { INTERNAL_NAME_MAX, internalName, specialHeader } from "./wizardSpecs";
import type { HeaderChoiceValue } from "./wizardSpecs";
import styles from "./WizardStep.module.css";

interface Props {
  headers: HeaderSummary[];
  links: WarpLink[];
  counts: MemberCounts;
  value: HeaderChoiceValue;
  /** The event file a copy would be made of (null: there is none to copy). */
  copySource: number | null;
  onChange: (value: HeaderChoiceValue) => void;
  /** The state of the Dynamic Headers patch; null until it is read (or when reading it failed: see `patchError`). */
  patch: HeaderPatchDto | null;
  /** Why the state could not be read, or why the patch could not be applied (the backend's text). */
  patchError: string | null;
  /** The patch is being applied. */
  patchBusy: boolean;
  /** Applies the patch; it reports its failure through `patchError`. */
  onApplyPatch: () => Promise<void>;
  /** Repairs a patch whose routine calls a wrong function (`patch.repairable`); reports through `patchError`. */
  onRepairPatch: () => Promise<void>;
  /** The header a new header is copied from. */
  template: number;
  /** The internal name a new header gets unless one is typed. */
  defaultName: string;
}

type Mode = "existing" | "new";
type EventFileKey = "Keep" | "New" | "Copy";

/** The usage a header row shows: how many warps lead there, and whether that makes it look free (a special header never does). */
function usage(h: HeaderSummary, incoming: number): string {
  if (specialHeader(h) !== null) return `Warps hierher: ${incoming}, Sonder-${termLabel("header")}, nicht frei`;
  return incoming === 0 ? "Warps hierher: 0, wirkt unbenutzt" : `Warps hierher: ${incoming}`;
}

/**
 * The step "Header wählen": which header the new matrix is given to, with its area data and event file. Three columns: the kind of
 * header (an existing one or a new one, with the Dynamic Headers patch it needs), the header itself, and the data it gets.
 */
export function HeaderChoice({ headers, links, counts, value, copySource, onChange, patch, patchError, patchBusy, onApplyPatch, onRepairPatch, template, defaultName }: Props) {
  const [picking, setPicking] = useState(false);
  const chosen = value.header === null ? null : (headers.find((h) => h.id === value.header) ?? null);
  const creating = value.newHeader !== null;
  const templateHeader = headers.find((h) => h.id === template);
  // The number the new header gets: the next one.
  const newId = headers.length;
  // Warps that lead to each header: a header none leads to is not reachable in the game.
  const incoming = useMemo(() => {
    const counts = new Map<number, number>();
    for (const link of links) counts.set(link.dest_header_id, (counts.get(link.dest_header_id) ?? 0) + 1);
    return counts;
  }, [links]);
  const incomingCount = (id: number) => incoming.get(id) ?? 0;
  const describe = useCallback((h: HeaderSummary) => usage(h, incoming.get(h.id) ?? 0), [incoming]);
  const areaMax = Math.max(0, (counts.AreaData ?? 1) - 1);
  const set = (change: Partial<HeaderChoiceValue>) => onChange({ ...value, ...change });

  // A patch that keeps the game from starting is repaired first.
  const canCreate = patch !== null && patch.supported && patch.applied && !patch.repairable;
  const unsupported = patch === null ? patchError !== null : !patch.supported;
  // Area data and event file belong to a header: without one only the matrix is made.
  const noHeader = chosen === null && !creating;

  const modes: SegmentedOption<Mode>[] = [
    { value: "existing", label: `Vorhandenen ${termLabel("header")} umstellen` },
    { value: "new", label: `Neuer ${termLabel("header")}`, disabled: !canCreate },
  ];
  const eventFiles: SegmentedOption<EventFileKey>[] = [
    { value: "Keep", label: "Behalten" },
    { value: "New", label: "Neu, leer" },
    { value: "Copy", label: copySource === null ? "Kopie" : `Kopie von ${id4(copySource)}`, disabled: copySource === null },
  ];
  const eventFileKey: EventFileKey = typeof value.eventFile === "object" ? "Copy" : value.eventFile;
  const chooseEventFile = (key: EventFileKey) => {
    let choice: EventFileChoice;
    if (key === "Copy") {
      if (copySource === null) return;
      choice = { Copy: copySource };
    } else choice = key;
    set({ eventFile: choice });
  };

  return (
    <>
      <section className={styles.column}>
        <h3 className={styles.heading}>{termLabel("header")}</h3>
        <div className={styles.mode}>
          <Segmented
            ariaLabel={`Art des ${termLabel("header")}s`}
            options={modes}
            value={creating ? "new" : "existing"}
            onChange={(mode) => set(mode === "new" ? { header: null, newHeader: { template, internalName: defaultName } } : { newHeader: null })}
          />
          {unsupported && <Badge kind="ptHgssOnly" />}
        </div>
        {patch === null && patchError === null && <p className={styles.note}>Prüfe, ob neue {termPlural("header")} möglich sind …</p>}
        {unsupported && <p className={styles.note}>Neue {termPlural("header")} gibt es nur für Platin und HeartGold/SoulSilver.</p>}
        {patch?.supported === true && !patch.applied && (
          <Notice kind="warning" title={`${termLabel("patch")} „Dynamische Header“ nötig`}>
            <p className={styles.text}>
              Der Patch ändert Bytes im Spielcode (arm9.bin). Die {termPlural("header")} liegen danach in einer Datei je {termLabel("header")}. Das lässt sich nicht zurücknehmen; die Sicherung
              arm9.bin.backup wird angelegt. DSPRE kennt den Patch und zeigt ihn als angewendet.
            </p>
            <div className={styles.actions}>
              <Button kind="secondary" size="s" disabled={patchBusy} onClick={() => void onApplyPatch()}>
                Patch anwenden
              </Button>
              {patchBusy && <span className={styles.note}>Wende an …</span>}
            </div>
          </Notice>
        )}
        {patch?.applied === true && patch.repairable && (
          <Notice kind="error" title={`${termLabel("patch")} „Dynamische Header“ fehlerhaft`}>
            <p className={styles.text}>
              Der Patch ruft im Spielcode eine falsche Funktion auf: das Spiel bleibt nach dem Intro schwarz. So haben ihn DSPRE und ältere Studio-Versionen für Platin (deutsch, französisch,
              italienisch, spanisch) geschrieben. Die Reparatur ändert 4 Bytes in arm9.bin; die {termPlural("header")} bleiben, wie sie sind.
            </p>
            <div className={styles.actions}>
              <Button kind="secondary" size="s" disabled={patchBusy} onClick={() => void onRepairPatch()}>
                Patch reparieren
              </Button>
              {patchBusy && <span className={styles.note}>Repariere …</span>}
            </div>
          </Notice>
        )}
        {patchError !== null && <Notice kind="error">{patchError}</Notice>}
      </section>

      {creating && value.newHeader !== null ? (
        <section className={styles.column}>
          <h3 className={styles.heading}>Neuer {termLabel("header")}</h3>
          <p className={styles.text}>
            Kopie von {termLabel("header")} {id4(template)}
            {templateHeader ? ` „${headerLabel(templateHeader)}“` : ""}
          </p>
          <div className={styles.field}>
            <TextField
              label="Interner Name"
              showLabel
              value={value.newHeader.internalName}
              onCommit={(text) => set({ newHeader: { template, internalName: internalName(text) } })}
              hint={`Wird ${termLabel("header")} ${id4(newId)}; höchstens ${INTERNAL_NAME_MAX} Zeichen: A bis Z, 0 bis 9 und _.`}
            />
          </div>
        </section>
      ) : (
        <section className={styles.column}>
          <h3 className={styles.heading}>Gewählt</h3>
          {chosen ? (
            <>
              <div className={styles.chip}>
                <Reference area="welt" label={headerLabel(chosen)} tech={id4(chosen.id)} onOpen={() => setPicking(true)} dataAttrs={{ "data-header": chosen.id }} />
              </div>
              <p className={styles.text}>Warps hierher: {incomingCount(chosen.id)}</p>
              {specialHeader(chosen) !== null ? (
                <Notice kind="warning">
                  Sonder-{termLabel("header")}: {specialHeader(chosen)}. Nicht frei, auch wenn kein Warp hierher führt; einen anderen oder „Neuer {termLabel("header")}“ wählen.
                </Notice>
              ) : (
                incomingCount(chosen.id) === 0 && <p className={styles.warn}>wirkt unbenutzt: nichts führt hierher</p>
              )}
            </>
          ) : (
            <p className={styles.note}>
              Kein {termLabel("header")} gewählt: nur das {termLabel("matrix")} wird angelegt.
            </p>
          )}
          <div className={styles.actions}>
            <Button kind="secondary" size="s" icon="search" onClick={() => setPicking(true)}>
              {termLabel("header")} wählen…
            </Button>
            <Button kind="quiet" size="s" disabled={value.header === null} onClick={() => set({ header: null })}>
              Keinen {termLabel("header")} umstellen
            </Button>
          </div>
          <HeaderPicker
            open={picking}
            onOpenChange={setPicking}
            headers={headers}
            onPick={(id) => set({ header: id })}
            title={`${termLabel("header")} wählen`}
            describe={describe}
          />
        </section>
      )}

      <section className={styles.column}>
        <h3 className={styles.heading}>Daten des {termLabel("header")}s</h3>
        <div className={styles.field}>
          <NumberField
            label={termLabel("areaData")}
            showLabel
            min={0}
            max={areaMax}
            value={value.areaDataId}
            disabled={noHeader}
            onCommit={(n) => set({ areaDataId: n })}
            onClear={() => set({ areaDataId: null })}
            hint={`0 bis ${areaMax}; leer lassen behält die des ${termLabel("header")}s.`}
          />
        </div>
        <div className={styles.group}>
          <span className={styles.label}>{termLabel("eventFile")}</span>
          <Segmented ariaLabel={termLabel("eventFile")} options={eventFiles.map((o) => (noHeader ? { ...o, disabled: true } : o))} value={eventFileKey} onChange={chooseEventFile} />
          {creating && <p className={styles.note}>Behalten teilt die {termLabel("eventFile")} mit {termLabel("header")} {id4(template)}.</p>}
          {copySource === null && <p className={styles.note}>Die Quelle hat keine {termLabel("eventFile")} zum Kopieren.</p>}
        </div>
      </section>
    </>
  );
}
