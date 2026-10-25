import { useEffect, useState } from "react";
import { errorText, exportMapPart } from "../../api";
import type { MapPart } from "../../api";
import { savePath } from "../../platform/dialogs";
import { inTauri } from "../../platform/transport";
import { Button, Dialog, Notice, Segmented, TextField } from "../../ui/primitives";
import { id4 } from "../terms";
import { PART_NAME } from "./importParts";
import styles from "./ImportPartsDialog.module.css";

const EXTENSION: Record<MapPart, string> = { model: "nsbmd", permissions: "per", buildings: "bld", bdhc: "bdhc", bgs: "bgs", whole: "bin" };
const PARTS: readonly MapPart[] = ["model", "permissions", "buildings", "bdhc", "bgs", "whole"];
const KNOWN = new Set(Object.values(EXTENSION));

/** The default file of a part: `karte_0006.per`. */
const defaultName = (map: number, part: MapPart) => `karte_${id4(map)}.${EXTENSION[part]}`;

/**
 * `path` for another part: a typed file keeps its folder and name and takes the part's ending (an ending of another part is replaced,
 * any other kept), a folder alone gets the default name, an empty field stays empty (its placeholder shows the default).
 */
function pathFor(path: string, map: number, part: MapPart): string {
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

/** „Teile exportieren…“: writes one part of the map (unsaved edits included) to a file. */
export function ExportPartDialog({ map, open, onClose }: ExportPartDialogProps) {
  const [part, setPart] = useState<MapPart>("model");
  const [path, setPath] = useState("");
  const [working, setWorking] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  // What the last export wrote: the notice names that part and path, and goes when either changes.
  const [written, setWritten] = useState<{ part: MapPart; path: string } | null>(null);

  useEffect(() => {
    if (!open) return;
    setPart("model");
    setPath("");
    setFailure(null);
    setWritten(null);
  }, [open, map]);

  async function run() {
    const target = path.trim();
    if (target === "") return;
    setWorking(true);
    setFailure(null);
    setWritten(null);
    try {
      await exportMapPart(map, part, target);
      setWritten({ part, path: target });
    } catch (e) {
      setFailure(failureText(e, target));
    } finally {
      setWorking(false);
    }
  }

  function choosePart(next: MapPart) {
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
        <Segmented ariaLabel="Teil" options={PARTS.map((p) => ({ value: p, label: PART_NAME[p] }))} value={part} onChange={choosePart} size="s" wrap />
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
        {failure !== null && <Notice kind="error">{failure}</Notice>}
        {written !== null && <Notice kind="success">{`${PART_NAME[written.part]} exportiert nach ${written.path}`}</Notice>}
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
