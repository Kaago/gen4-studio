import { useMemo } from "react";
import type { CSSProperties } from "react";
import type { EventFileChoice, HeaderPatchDto, HeaderSummary, MemberCounts, WarpLink } from "../../api";
import { DraftInput } from "../DraftInput";
import { headerLabel } from "../headers";
import { HeaderPicker } from "../inspector/HeaderPicker";
import { parseCellField } from "../matrix";
import { TERMS, id4 } from "../terms";
import { INTERNAL_NAME_MAX, internalName, specialHeader } from "./wizardSpecs";
import type { HeaderChoiceValue } from "./wizardSpecs";

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

const row: CSSProperties = { display: "flex", alignItems: "center", gap: 8, margin: "6px 0" };
const heading: CSSProperties = { fontWeight: 600, marginBottom: 4 };
const box: CSSProperties = { border: "1px solid #d97706", background: "#fffbeb", borderRadius: 6, padding: "8px 12px", margin: "8px 0" };
const muted: CSSProperties = { color: "#6b7280", margin: "6px 0" };
const NAME_PATTERN = "[A-Z0-9_]*";

/** The usage a header row shows: how many warps lead there, and whether that makes it look free (a special header never does). */
function usage(h: HeaderSummary, incoming: number): string {
  if (specialHeader(h) !== null) return `Warps hierher: ${incoming}, Sonder-${TERMS.header.label}, nicht frei`;
  return incoming === 0 ? "Warps hierher: 0, wirkt unbenutzt" : `Warps hierher: ${incoming}`;
}

/** The step "Header wählen": which header the new matrix is given to, with its area data and event file. */
export function HeaderChoice({ headers, links, counts, value, copySource, onChange, patch, patchError, patchBusy, onApplyPatch, onRepairPatch, template, defaultName }: Props) {
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
  const areaMax = Math.max(0, (counts.AreaData ?? 1) - 1);
  const set = (change: Partial<HeaderChoiceValue>) => onChange({ ...value, ...change });
  const eventFile = (choice: EventFileChoice) => set({ eventFile: choice });
  const isCopy = typeof value.eventFile === "object";

  // A patch that keeps the game from starting is repaired first.
  const canCreate = patch !== null && patch.supported && patch.applied && !patch.repairable;

  return (
    <div>
      <div>
        <label style={row}>
          <input type="radio" name="header-mode" checked={!creating} onChange={() => set({ newHeader: null })} />
          Vorhandenen {TERMS.header.label} umstellen
        </label>
        <label style={row}>
          <input
            type="radio"
            name="header-mode"
            disabled={!canCreate}
            checked={creating}
            onChange={() => set({ header: null, newHeader: { template, internalName: defaultName } })}
          />
          Neuer {TERMS.header.label}
        </label>
      </div>
      {patch === null && patchError === null && <div style={muted}>Prüfe, ob neue {TERMS.header.plural} möglich sind …</div>}
      {(patch === null ? patchError !== null : !patch.supported) && (
        <div style={muted}>
          Neue {TERMS.header.plural} gibt es nur für Platin und HeartGold/SoulSilver.
          {patchError !== null && <div style={{ color: "#b91c1c" }}>{patchError}</div>}
        </div>
      )}
      {patch?.supported === true && !patch.applied && (
        <div style={box}>
          <div style={heading}>{TERMS.patch.label} „Dynamische Header“ nötig</div>
          <p style={{ margin: "4px 0" }}>
            Der Patch ändert Bytes im Spielcode (arm9.bin). Die {TERMS.header.plural} liegen danach in einer Datei je {TERMS.header.label}. Das lässt sich nicht zurücknehmen; die Sicherung
            arm9.bin.backup wird angelegt. DSPRE kennt den Patch und zeigt ihn als angewendet.
          </p>
          <button type="button" disabled={patchBusy} onClick={() => void onApplyPatch()}>
            Patch anwenden
          </button>
          {patchBusy && <span style={{ marginLeft: 8, color: "#6b7280" }}>Wende an …</span>}
          {patchError !== null && <div style={{ color: "#b91c1c", marginTop: 6 }}>{patchError}</div>}
        </div>
      )}
      {patch?.applied === true && patch.repairable && (
        <div style={{ ...box, borderColor: "#b91c1c", background: "#fef2f2" }}>
          <div style={heading}>{TERMS.patch.label} „Dynamische Header“ fehlerhaft</div>
          <p style={{ margin: "4px 0" }}>
            Der Patch ruft im Spielcode eine falsche Funktion auf: das Spiel bleibt nach dem Intro schwarz. So haben ihn DSPRE und ältere Studio-Versionen für Platin (deutsch, französisch,
            italienisch, spanisch) geschrieben. Die Reparatur ändert 4 Bytes in arm9.bin; die {TERMS.header.plural} bleiben, wie sie sind.
          </p>
          <button type="button" disabled={patchBusy} onClick={() => void onRepairPatch()}>
            Patch reparieren
          </button>
          {patchBusy && <span style={{ marginLeft: 8, color: "#6b7280" }}>Repariere …</span>}
          {patchError !== null && <div style={{ color: "#b91c1c", marginTop: 6 }}>{patchError}</div>}
        </div>
      )}

      {creating && value.newHeader !== null ? (
        <div style={{ margin: "8px 0" }}>
          <div>
            Kopie von {TERMS.header.label} {id4(template)}
            {templateHeader ? ` „${headerLabel(templateHeader)}“` : ""}
          </div>
          <label style={row}>
            Interner Name
            <DraftInput
              label="Interner Name"
              value={value.newHeader.internalName}
              maxLength={INTERNAL_NAME_MAX}
              pattern={NAME_PATTERN}
              onCommit={(text) => set({ newHeader: { template, internalName: internalName(text) } })}
              style={{ width: 200, fontFamily: "ui-monospace, monospace" }}
            />
          </label>
          <div style={muted}>
            Wird {TERMS.header.label} {id4(newId)}; höchstens {INTERNAL_NAME_MAX} Zeichen: A bis Z, 0 bis 9 und _.
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", gap: 16 }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={heading}>{TERMS.header.label} wählen</div>
            <HeaderPicker
              headers={headers}
              current={value.header ?? -1}
              onPick={(id) => set({ header: id })}
              renderExtra={(h) => usage(h, incomingCount(h.id))}
            />
            <button type="button" disabled={value.header === null} onClick={() => set({ header: null })} style={{ marginTop: 4 }}>
              Keinen {TERMS.header.label} umstellen
            </button>
          </div>
          <div style={{ width: 200, flexShrink: 0 }}>
            <div style={heading}>Gewählt</div>
            {chosen ? (
              <>
                <div>
                  {TERMS.header.label} {id4(chosen.id)} „{headerLabel(chosen)}“
                </div>
                <div>Warps hierher: {incomingCount(chosen.id)}</div>
                {specialHeader(chosen) !== null ? (
                  <div style={{ color: "#b91c1c" }}>
                    Sonder-{TERMS.header.label}: {specialHeader(chosen)}. Nicht frei, auch wenn kein Warp hierher führt; einen anderen oder „Neuer {TERMS.header.label}“ wählen.
                  </div>
                ) : (
                  incomingCount(chosen.id) === 0 && <div style={{ color: "#b45309" }}>wirkt unbenutzt: nichts führt hierher</div>
                )}
              </>
            ) : (
              <div style={{ color: "#6b7280" }}>Kein {TERMS.header.label} gewählt: nur das {TERMS.matrix.label} wird angelegt.</div>
            )}
          </div>
        </div>
      )}

      <fieldset disabled={chosen === null && !creating} style={{ border: "none", padding: 0, margin: "12px 0 0" }}>
        <label style={row}>
          {TERMS.areaData.label}
          <DraftInput
            label={`${TERMS.areaData.label} (0 bis ${areaMax}, leer: behalten)`}
            type="number"
            min={0}
            max={areaMax}
            value={value.areaDataId === null ? "" : String(value.areaDataId)}
            onCommit={(text) => {
              const n = parseCellField(text, areaMax, true);
              if (n !== null) set({ areaDataId: n === "empty" ? null : n });
            }}
            style={{ width: 80, textAlign: "right", fontFamily: "ui-monospace, monospace" }}
          />
        </label>
        <div style={heading}>{TERMS.eventFile.label}</div>
        <label style={row}>
          <input type="radio" name="event-file" checked={value.eventFile === "Keep"} onChange={() => eventFile("Keep")} />
          {creating ? `behalten (geteilt mit ${TERMS.header.label} ${id4(template)})` : "behalten"}
        </label>
        <label style={row}>
          <input type="radio" name="event-file" checked={value.eventFile === "New"} onChange={() => eventFile("New")} />
          neu, leer
        </label>
        <label style={row}>
          <input type="radio" name="event-file" disabled={copySource === null} checked={isCopy} onChange={() => copySource !== null && eventFile({ Copy: copySource })} />
          {copySource === null ? "Kopie (die Quelle hat keine)" : `Kopie von ${id4(copySource)}`}
        </label>
      </fieldset>
    </div>
  );
}
