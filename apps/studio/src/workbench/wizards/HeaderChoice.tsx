import type { CSSProperties } from "react";
import type { EventFileChoice, HeaderSummary, MemberCounts, WarpLink } from "../../api";
import { DraftInput } from "../DraftInput";
import { headerLabel } from "../headers";
import { HeaderPicker } from "../inspector/HeaderPicker";
import { parseCellField } from "../matrix";
import { TERMS, id4 } from "../terms";
import type { HeaderChoiceValue } from "./wizardSpecs";

interface Props {
  headers: HeaderSummary[];
  links: WarpLink[];
  counts: MemberCounts;
  value: HeaderChoiceValue;
  /** The event file a copy would be made of (null: there is none to copy). */
  copySource: number | null;
  onChange: (value: HeaderChoiceValue) => void;
}

const row: CSSProperties = { display: "flex", alignItems: "center", gap: 8, margin: "6px 0" };
const heading: CSSProperties = { fontWeight: 600, marginBottom: 4 };

/** The step "Header wählen": which header the new matrix is given to, with its area data and event file. */
export function HeaderChoice({ headers, links, counts, value, copySource, onChange }: Props) {
  const chosen = value.header === null ? null : (headers.find((h) => h.id === value.header) ?? null);
  const incoming = chosen ? links.filter((l) => l.dest_header_id === chosen.id).length : 0;
  const areaMax = Math.max(0, (counts.AreaData ?? 1) - 1);
  const set = (change: Partial<HeaderChoiceValue>) => onChange({ ...value, ...change });
  const eventFile = (choice: EventFileChoice) => set({ eventFile: choice });
  const isCopy = typeof value.eventFile === "object";

  return (
    <div>
      <div style={{ display: "flex", gap: 16 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={heading}>{TERMS.header.label} wählen</div>
          <HeaderPicker headers={headers} current={value.header ?? -1} onPick={(id) => set({ header: id })} />
          <button type="button" disabled={value.header === null} onClick={() => set({ header: null })} style={{ marginTop: 4 }}>
            Keinen {TERMS.header.label} umstellen
          </button>
        </div>
        <div style={{ width: 200, flexShrink: 0 }}>
          <div style={heading}>Warps hierher</div>
          {chosen ? (
            <>
              <div>
                {TERMS.header.label} {id4(chosen.id)} „{headerLabel(chosen)}“
              </div>
              <div>{incoming}</div>
              {incoming === 0 && <div style={{ color: "#b45309" }}>wirkt unbenutzt: nichts führt hierher</div>}
            </>
          ) : (
            <div style={{ color: "#6b7280" }}>Kein {TERMS.header.label} gewählt: nur das {TERMS.matrix.label} wird angelegt.</div>
          )}
        </div>
      </div>

      <fieldset disabled={chosen === null} style={{ border: "none", padding: 0, margin: "12px 0 0" }}>
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
          behalten
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
