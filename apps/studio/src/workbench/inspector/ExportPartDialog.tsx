import { useEffect, useState } from "react";
import { errorText, exportMapPart, worldExportTmj, worldGet } from "../../api";
import type { MapPart } from "../../api";
import { savePath } from "../../platform/dialogs";
import { inTauri } from "../../platform/transport";
import { Button, Dialog, Notice, Segmented, TextField } from "../../ui/primitives";
import { id4 } from "../terms";
import { PART_NAME } from "./importParts";
import styles from "./ImportPartsDialog.module.css";

/** The parts a map file holds (textures live in the area's map texture pack and are not exported), and a world map as a Tiled map (6.5b). */
type FilePart = Exclude<MapPart, "textures"> | "tmj";

const EXTENSION: Record<FilePart, string> = { model: "nsbmd", permissions: "per", buildings: "bld", bdhc: "bdhc", bgs: "bgs", whole: "bin", tmj: "tmj" };
const PARTS: readonly FilePart[] = ["model", "permissions", "buildings", "bdhc", "bgs", "whole"];
const WORLD_PARTS: readonly FilePart[] = [...PARTS, "tmj"];
const KNOWN = new Set(Object.values(EXTENSION));
const NAME: Record<FilePart, string> = { ...PART_NAME, tmj: "Tiled (TMJ)" };

/** The default file of a part: `karte_0006.per`; a Tiled map `0006.tmj`. */
const defaultName = (map: number, part: FilePart) => (part === "tmj" ? `${id4(map)}.tmj` : `karte_${id4(map)}.${EXTENSION[part]}`);

/** The folder of the tile images next to a Tiled map: `<stem>.tiles` of the typed file, `0006.tiles` before one is typed. */
function tilesFolder(path: string, map: number): string {
  const trimmed = path.trim();
  const name = trimmed.slice(Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\")) + 1);
  const dot = name.lastIndexOf(".");
  const stem = dot > 0 ? name.slice(0, dot) : name;
  return `${stem === "" ? id4(map) : stem}.tiles`;
}

/**
 * `path` for another part: a typed file keeps its folder and name and takes the part's ending (an ending of another part is replaced,
 * any other kept), a folder alone gets the default name, an empty field stays empty (its placeholder shows the default).
 */
function pathFor(path: string, map: number, part: FilePart): string {
  const trimmed = path.trim();
  if (trimmed === "") return path;
  const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  const folder = trimmed.slice(0, cut + 1);
  const name = trimmed.slice(cut + 1);
  if (name === "") return folder + defaultName(map, part);
  const dot = name.lastIndexOf(".");
  const stem = dot > 0 && KNOWN.has(name.slice(dot + 1).toLowerCase()) ? name.slice(0, dot) : name;
  return `${folder}${stem}.${EXTENSION[part]}`;
}

/**
 * The sentence for a failed export. A folder on the way that does not exist (the bridge finds it when it checks the folder, the app's
 * kernel when it writes; both say it in English with the system's „os error 2“, Windows also 3) is said in German with the way out.
 */
function failureText(e: unknown, target: string): string {
  const message = errorText(e);
  const cut = Math.max(target.lastIndexOf("/"), target.lastIndexOf("\\"));
  if (cut > 0 && /\(os error [23]\)\s*$/.test(message)) {
    return `Den Ordner ${target.slice(0, cut)} gibt es nicht. Lege ihn an oder wähle einen anderen Pfad.`;
  }
  return message;
}

export interface ExportPartDialogProps {
  map: number;
  open: boolean;
  onClose: () => void;
}

/**
 * „Teile exportieren…“: writes one part of the map (unsaved edits included) to a file. A world map (the dialog asks `world_get` when it
 * opens) also offers „Tiled (TMJ)“ (`world_export_tmj`, the tile images in `<stem>.tiles` next to it).
 */
export function ExportPartDialog({ map, open, onClose }: ExportPartDialogProps) {
  const [chosenPart, setPart] = useState<FilePart>("model");
  const [path, setPath] = useState("");
  const [working, setWorking] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  // What the last export wrote: the notice names that part and path, and goes when either changes.
  const [written, setWritten] = useState<{ part: FilePart; path: string } | null>(null);
  // Whether the map is a world map, for the map it belongs to.
  const [worldOf, setWorldOf] = useState<{ map: number; world: boolean } | null>(null);

  useEffect(() => {
    if (!open) return;
    setPart("model");
    setPath("");
    setFailure(null);
    setWritten(null);
  }, [open, map]);

  useEffect(() => {
    if (!open) return;
    let current = true;
    worldGet(map).then(
      (dto) => current && setWorldOf({ map, world: dto.world !== null }),
      () => current && setWorldOf({ map, world: false }),
    );
    return () => {
      current = false;
    };
  }, [open, map]);
  const world = worldOf?.map === map && worldOf.world;
  // A Tiled map only for a world map: a map that is none (any more) goes back to the model.
  const part: FilePart = chosenPart === "tmj" && !world ? "model" : chosenPart;

  async function run() {
    const target = path.trim();
    if (target === "") return;
    setWorking(true);
    setFailure(null);
    setWritten(null);
    try {
      if (part === "tmj") await worldExportTmj(map, target);
      else await exportMapPart(map, part, target);
      setWritten({ part, path: target });
    } catch (e) {
      setFailure(failureText(e, target));
    } finally {
      setWorking(false);
    }
  }

  function choosePart(next: FilePart) {
    setPart(next);
    setPath((current) => pathFor(current, map, next));
    setFailure(null);
    setWritten(null);
  }

  function choosePath(next: string) {
    setPath(next);
    setFailure(null);
    setWritten(null);
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()} title={`Teil von Karte ${id4(map)} exportieren`}>
      <div className={styles.body} data-export-dialog={map}>
        <div data-export-part={part}>
          <Segmented ariaLabel="Teil" options={(world ? WORLD_PARTS : PARTS).map((p) => ({ value: p, label: NAME[p] }))} value={part} onChange={choosePart} size="s" wrap />
        </div>
        <div className={styles.source}>
          <TextField label="Pfad" showLabel value={path} onCommit={choosePath} placeholder={`/Pfad/${defaultName(map, part)}`} />
          {inTauri() && (
            <Button
              kind="secondary"
              size="s"
              onClick={() =>
                void savePath({ defaultPath: defaultName(map, part) }).then((chosen) => {
                  if (chosen) choosePath(chosen);
                })
              }
            >
              Pfad wählen…
            </Button>
          )}
        </div>
        {part === "tmj" && <p className={styles.reason}>{`Kachelbilder kommen in den Ordner ${tilesFolder(path, map)} daneben.`}</p>}
        {failure !== null && <Notice kind="error">{failure}</Notice>}
        {written !== null && <Notice kind="success">{`${NAME[written.part]} exportiert nach ${written.path}`}</Notice>}
        <div className={styles.actions}>
          <Button kind="secondary" onClick={onClose}>
            Schließen
          </Button>
          <Button kind="primary" disabled={path.trim() === "" || working} onClick={() => void run()}>
            Exportieren
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
