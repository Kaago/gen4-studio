import { useEffect, useState } from "react";
import type { JSX } from "react";
import { recipeApply, tilesetGet } from "../api";
import type { DraftDto, MatrixDto, NewPaintedMapInput, NewPaintedMapReport, TilesetDto } from "../api";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { Dialog, NumberField, TextField } from "../ui/primitives";
import { TilePalette } from "./inspector/TilePalette";
import styles from "./NewPaintedMapDialog.module.css";
import { DraftIrreversible, DraftNotes } from "./wizards/DraftNotes";
import { useDraftPlan } from "./wizards/useDraftPlan";
import { useStartPainting } from "./WorkbenchContext";

/** The height of the ground in steps (the kernel's range). */
const HEIGHT_MIN = -16;
const HEIGHT_MAX = 62;

interface Props {
  matrix: MatrixDto;
  x: number;
  y: number;
  /** Tüfteln adds „Tileset“ and „Header“ and shows the kernel's notes. */
  depth: Depth;
  /** The map was created (`report`), or the dialog was left without a result (null: cancelled, or it failed, see `onError`). */
  onDone: (report: { map: number } | null) => void;
  onError: (e: unknown) => void;
  /** The draft of the new cell as planned (null: nothing planned): the scene and the world map draw it until the dialog goes. */
  onDraft: (draft: DraftDto | null) => void;
  /** Where the palette's pictures come from (tests have none). */
  image?: (pkg: number, texture: string) => Promise<ImageBitmap | null>;
}

/** The tileset of package `pkg` (null: none asked for, or not read yet); a failed read leaves the palette out. */
function useTileset(pkg: number | null): TilesetDto | null {
  const [shown, setShown] = useState<TilesetDto | null>(null);
  useEffect(() => {
    if (pkg === null) return;
    let live = true;
    tilesetGet(pkg).then(
      (dto) => live && setShown(dto),
      () => live && setShown(null),
    );
    return () => {
      live = false;
    };
  }, [pkg]);
  return shown !== null && shown.package === pkg ? shown : null;
}

/**
 * „Neue Karte malen“ (recipe `new_painted_map`, text project): a new world map at the cell `(x, y)` of `matrix` with a blank world of
 * one ground tile at one height, reached from the world map's „+“ („Stattdessen neu malen“). The name is required; „Boden“ is the
 * palette of the draft's package, the draft's tile chosen until another is; Tüfteln may set package and header. The draft is planned
 * once there is a name and again with every change. „Karte malen“ applies it, then the app selects the cell and starts painting.
 */
export function NewPaintedMapDialog({ matrix, x, y, depth, onDone, onError, onDraft, image }: Props): JSX.Element {
  const tinker = depthAtLeast(depth, "tuefteln");
  const [name, setName] = useState("");
  const [fill, setFill] = useState<string | null>(null);
  const [height, setHeight] = useState(0);
  const [tileset, setTileset] = useState<number | null>(null);
  const [header, setHeader] = useState<number | null>(null);
  const [working, setWorking] = useState(false);
  const startPainting = useStartPainting();

  const named = name.trim();
  const input: NewPaintedMapInput | null =
    named === ""
      ? null
      : {
          matrix: matrix.index,
          x,
          y,
          name: named,
          ...(tileset !== null && { tileset }),
          ...(fill !== null && { fill_tile: fill }),
          height,
          ...(header !== null && { header }),
        };
  const plan = useDraftPlan("new_painted_map", input, onDraft);
  const painted = plan.draft?.painted ?? null;
  const palette = useTileset(painted?.tileset ?? null);

  async function paint() {
    if (working || input === null) return;
    setWorking(true);
    let report: NewPaintedMapReport;
    try {
      report = (await recipeApply<NewPaintedMapReport>("new_painted_map", input)).report;
    } catch (e) {
      onError(e);
      onDone(null);
      return;
    }
    onDone(report);
    startPainting({ matrix: report.matrix, x: report.x, y: report.y });
  }

  const leave = () => {
    if (!working) onDone(null);
  };
  // Another package or header may have other tiles: the ground goes back to the draft's.
  const chooseTileset = (pkg: number) => {
    setTileset(pkg);
    setFill(null);
  };
  const chooseHeader = (id: number) => {
    setHeader(id);
    setFill(null);
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) leave();
      }}
      title="Neue Karte malen"
      actions={[
        { label: "Abbrechen", onClick: leave },
        { label: "Karte malen", kind: "primary", onClick: () => void paint(), disabled: input === null || working },
      ]}
    >
      <div className={styles.form} aria-busy={working}>
        <TextField label="Name" showLabel hint="Pflicht" value={name} disabled={working} onCommit={setName} />
        {palette && (
          <div className={styles.ground} role="group" aria-label="Boden" data-paint-ground="">
            <p className={styles.label}>Boden</p>
            <TilePalette tileset={palette} tile={fill ?? painted?.fill_tile ?? ""} onTile={setFill} depth={depth} image={image} />
          </div>
        )}
        <NumberField label="Höhe" showLabel hint="Stufen, −16 bis 62" min={HEIGHT_MIN} max={HEIGHT_MAX} value={height} disabled={working} onCommit={setHeight} />
        {tinker && (
          <>
            <NumberField label="Tileset" showLabel min={0} value={tileset ?? painted?.tileset ?? null} disabled={working} onCommit={chooseTileset} />
            <NumberField label="Header" showLabel min={0} value={header ?? painted?.header ?? null} disabled={working} onCommit={chooseHeader} />
          </>
        )}
        <DraftNotes draft={plan.draft} error={plan.error} depth={depth} />
        <DraftIrreversible draft={plan.draft} />
      </div>
    </Dialog>
  );
}
