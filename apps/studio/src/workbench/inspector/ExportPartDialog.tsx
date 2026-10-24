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
  const [written, setWritten] = useState<string | null>(null);

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
      setWritten(target);
    } catch (e) {
      setFailure(errorText(e));
    } finally {
      setWorking(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()} title={`Teil von Karte ${id4(map)} exportieren`}>
      <div className={styles.body} data-export-dialog={map}>
        <Segmented ariaLabel="Teil" options={PARTS.map((p) => ({ value: p, label: PART_NAME[p] }))} value={part} onChange={setPart} size="s" />
        <div className={styles.source}>
          <TextField label="Pfad" showLabel value={path} onCommit={setPath} placeholder={`/Pfad/karte_${id4(map)}.${EXTENSION[part]}`} />
          {inTauri() && (
            <Button
              kind="secondary"
              size="s"
              onClick={() =>
                void savePath({ defaultPath: `karte_${id4(map)}.${EXTENSION[part]}` }).then((chosen) => {
                  if (chosen) setPath(chosen);
                })
              }
            >
              Pfad wählen…
            </Button>
          )}
        </div>
        {failure !== null && <Notice kind="error">{failure}</Notice>}
        {written !== null && <Notice kind="success">{`${PART_NAME[part]} exportiert nach ${written}`}</Notice>}
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
