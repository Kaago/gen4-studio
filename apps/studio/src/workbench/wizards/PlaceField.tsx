import { useState } from "react";
import type { HeaderSummary, MatrixSummary } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Button, NumberField, Reference } from "../../ui/primitives";
import { headerLabel } from "../headers";
import { HeaderPicker } from "../inspector/HeaderPicker";
import { id4 } from "../terms";
import { matrixTiles } from "./wizardSpecs";
import parts from "./WizardStep.module.css";

/** The last tile of the largest matrix (255 maps of 32 tiles): the bound while the matrix of the place is not known. */
const MATRIX_LAST_TILE = 255 * 32 - 1;

/**
 * The place a recipe of Erzählen works in: its name as a chip of the world (Tüfteln adds the number), „Ort wählen…“ and the picker of
 * every place. `data-figure-place` carries the chosen header (empty without one).
 */
export function PlaceField({ headers, value, onChange, depth, title = "Ort wählen" }: { headers: HeaderSummary[]; value: number | null; onChange: (header: number) => void; depth: Depth; title?: string }) {
  const [picking, setPicking] = useState(false);
  const chosen = value === null ? null : (headers.find((h) => h.id === value) ?? null);
  return (
    <div className={parts.group} data-figure-place={value ?? ""}>
      <span className={parts.label}>Ort</span>
      {chosen ? (
        <div className={parts.chip}>
          <Reference area="welt" label={headerLabel(chosen)} tech={depthAtLeast(depth, "tuefteln") ? id4(chosen.id) : undefined} onOpen={() => setPicking(true)} />
        </div>
      ) : (
        <p className={parts.note}>Noch kein Ort gewählt.</p>
      )}
      <Button kind="secondary" size="s" icon="search" onClick={() => setPicking(true)}>
        Ort wählen…
      </Button>
      <HeaderPicker open={picking} onOpenChange={setPicking} headers={headers} onPick={onChange} title={title} />
    </div>
  );
}

/** The tiles of `header`'s matrix (x and z), when the overview knows it. */
export function placeTiles(header: HeaderSummary | null, matrices: readonly MatrixSummary[]): { x: number; z: number } | null {
  const matrix = header === null ? undefined : matrices.find((m) => m.index === header.matrix_id);
  return matrix ? matrixTiles(matrix) : null;
}

/** „Kachel x“ and „Kachel z“: a tile of the place's matrix, absolute; empty until one is typed or came from the scene. */
export function TileFields({ x, z, size, onChange }: { x: number | null; z: number | null; size: { x: number; z: number } | null; onChange: (tile: { x?: number; z?: number }) => void }) {
  const maxX = size ? size.x - 1 : MATRIX_LAST_TILE;
  const maxZ = size ? size.z - 1 : MATRIX_LAST_TILE;
  return (
    <>
      <div className={parts.field}>
        <NumberField label="Kachel x" showLabel min={0} max={maxX} value={x} hint={`0 bis ${maxX}`} onCommit={(n) => onChange({ x: n })} />
      </div>
      <div className={parts.field}>
        <NumberField label="Kachel z" showLabel min={0} max={maxZ} value={z} hint={`0 bis ${maxZ}`} onCommit={(n) => onChange({ z: n })} />
      </div>
    </>
  );
}
