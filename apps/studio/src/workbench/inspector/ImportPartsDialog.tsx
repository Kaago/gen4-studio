import { useEffect, useMemo, useState } from "react";
import { errorText, importMapParts, inspectMapImport } from "../../api";
import type { ImportSource, MapPart, PartReport } from "../../api";
import { openFiles } from "../../platform/dialogs";
import { inTauri } from "../../platform/transport";
import { Button, CheckList, Checkbox, Dialog, Notice, Picker, Segmented, Switch, TextField, checkNumber } from "../../ui/primitives";
import type { CheckRow, PickerItem } from "../../ui/primitives";
import { id4 } from "../terms";
import { PART_NAME, isForeignWhole, noteText, partsWord } from "./importParts";
import styles from "./ImportPartsDialog.module.css";

type Mode = "files" | "map";

export interface ImportPartsDialogProps {
  /** The map the parts go into. */
  map: number;
  open: boolean;
  onClose: () => void;
  /** Number of maps in the project (the maps the picker offers). */
  mapCount: number;
  /** An import changed the map: the app refreshes pictures, scene and the dirty count. */
  onImported: (map: number) => void | Promise<void>;
}

const MODES = [
  { value: "files", label: "Datei" },
  { value: "map", label: "Karte im Projekt" },
] as const;

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/**
 * „Teile importieren…“: parts from files or from another map of the project are checked first (`inspect_map_import`), each one with its
 * size and state; the chosen parts are written as one undo step (`import_map_parts`). A part with an error cannot be chosen.
 */
export function ImportPartsDialog({ map, open, onClose, mapCount, onImported }: ImportPartsDialogProps) {
  const [mode, setMode] = useState<Mode>("map");
  const [paths, setPaths] = useState("");
  const [sourceText, setSourceText] = useState("");
  const [sourceError, setSourceError] = useState<string | undefined>();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [reports, setReports] = useState<PartReport[] | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [chosen, setChosen] = useState<ReadonlySet<MapPart>>(new Set());
  const [keepBuildings, setKeepBuildings] = useState(false);
  const [working, setWorking] = useState(false);
  const [done, setDone] = useState<number | null>(null);

  // Every opening starts afresh.
  useEffect(() => {
    if (!open) return;
    setMode("map");
    setPaths("");
    setSourceText("");
    setSourceError(undefined);
    setReports(null);
    setFailure(null);
    setChosen(new Set());
    setKeepBuildings(false);
    setDone(null);
  }, [open, map]);

  const source = useMemo<ImportSource | null>(() => {
    if (mode === "files") {
      const files = paths
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l !== "");
      return files.length > 0 ? { files } : null;
    }
    const checked = checkNumber(sourceText, "dec", 0, Math.max(0, mapCount - 1));
    return sourceText.trim() !== "" && checked.ok ? { map: checked.value } : null;
  }, [mode, paths, sourceText, mapCount]);
  const sourceKey = source === null ? null : JSON.stringify(source);

  // The check runs whenever the source changes; an answer for an older source is dropped.
  useEffect(() => {
    setReports(null);
    setFailure(null);
    setChosen(new Set());
    setDone(null);
    if (!open || source === null) return;
    let current = true;
    inspectMapImport(map, source).then(
      (list) => {
        if (!current) return;
        setReports(list);
        setChosen(new Set(list.filter((r) => r.state.kind !== "error").map((r) => r.part)));
      },
      (e) => current && setFailure(errorText(e)),
    );
    return () => {
      current = false;
    };
  }, [open, map, sourceKey]);

  const foreignWhole = reports?.some((r) => isForeignWhole(r) && chosen.has(r.part)) ?? false;
  const count = chosen.size;

  const toggle = (part: MapPart, on: boolean) =>
    setChosen((prev) => {
      const next = new Set(prev);
      if (on) next.add(part);
      else next.delete(part);
      return next;
    });

  async function run() {
    if (source === null || count === 0) return;
    setWorking(true);
    setFailure(null);
    try {
      const parts = (reports ?? []).map((r) => r.part).filter((p) => chosen.has(p));
      const changed = await importMapParts(map, source, parts, foreignWhole && keepBuildings);
      setDone(changed.length > 0 ? parts.length : 0);
      if (changed.length > 0) await onImported(map);
    } catch (e) {
      setFailure(errorText(e));
    } finally {
      setWorking(false);
    }
  }

  const rows: CheckRow[] = (reports ?? []).map((r) => ({
    id: r.part,
    state: r.state.kind,
    text: (
      <div className={styles.part}>
        <Checkbox
          label={`${PART_NAME[r.part]} · ${r.size.toLocaleString("de-DE")} Bytes${r.file ? ` · ${fileName(r.file)}` : ""}`}
          ariaLabel={PART_NAME[r.part]}
          checked={chosen.has(r.part)}
          disabled={r.state.kind === "error" || done !== null}
          onChange={(on) => toggle(r.part, on)}
        />
        {r.state.kind !== "ok" && <p className={styles.reason}>{r.state.message}</p>}
        {r.notes.map((n) => (
          <p key={n} className={styles.reason}>
            {noteText(n)}
          </p>
        ))}
      </div>
    ),
  }));

  const mapItems = useMemo<PickerItem[]>(
    () => (pickerOpen ? Array.from({ length: mapCount }, (_, i) => ({ id: i, label: `Karte ${id4(i)}`, tech: String(i), area: "welt" as const, disabled: i === map })) : []),
    [pickerOpen, mapCount, map],
  );

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()} title={`Teile in Karte ${id4(map)} importieren`}>
      <div className={styles.body} data-import-dialog={map}>
        <Segmented ariaLabel="Quelle" options={MODES} value={mode} onChange={setMode} size="s" />
        {mode === "files" ? (
          <div className={styles.source}>
            <TextField label="Pfade, eine je Zeile" showLabel multiline value={paths} onCommit={setPaths} placeholder="/Pfad/zur/karte.nsbmd" />
            {inTauri() && (
              <Button
                kind="secondary"
                size="s"
                onClick={() =>
                  void openFiles({ title: "Teile wählen" }).then((files) => {
                    if (files && files.length > 0) setPaths(files.join("\n"));
                  })
                }
              >
                Dateien wählen…
              </Button>
            )}
          </div>
        ) : (
          <div className={styles.source}>
            <TextField
              label="Quellkarte"
              showLabel
              placeholder="Nummer"
              value={sourceText}
              error={sourceError}
              onChange={() => setSourceError(undefined)}
              onCommit={(text) => {
                const checked = checkNumber(text, "dec", 0, Math.max(0, mapCount - 1));
                setSourceError(text.trim() === "" || checked.ok ? undefined : checked.error);
                setSourceText(text);
              }}
            />
            <Button kind="secondary" size="s" onClick={() => setPickerOpen(true)}>
              Karte wählen…
            </Button>
            <Picker open={pickerOpen} onOpenChange={setPickerOpen} title="Quellkarte wählen" items={mapItems} placeholder="Nummer suchen" onPick={(item) => setSourceText(String(item.id))} />
          </div>
        )}
        {source !== null && reports === null && failure === null && <p className={styles.reason}>Teile werden geprüft…</p>}
        {reports !== null && reports.length === 0 && <p className={styles.reason}>Die Quelle enthält keine Teile.</p>}
        {rows.length > 0 && <CheckList ariaLabel="Teile" rows={rows} />}
        {foreignWhole && done === null && <Switch label="Gebäude übernehmen" checked={keepBuildings} onChange={setKeepBuildings} />}
        {failure !== null && <Notice kind="error">{failure}</Notice>}
        {done !== null && (
          <Notice kind={done > 0 ? "success" : "info"} actions={[{ label: "Schließen", onClick: onClose }]}>
            {done > 0 ? `${partsWord(done)} übernommen` : "Nichts geändert: die Teile waren schon so."}
          </Notice>
        )}
        {done === null && (
          <div className={styles.actions}>
            <Button kind="secondary" onClick={onClose}>
              Abbrechen
            </Button>
            <Button kind="primary" disabled={count === 0 || working || source === null} onClick={() => void run()}>
              {`${partsWord(count)} übernehmen`}
            </Button>
          </div>
        )}
      </div>
    </Dialog>
  );
}
