import { useEffect, useMemo, useRef, useState } from "react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { tilesetEdit, tilesetSave } from "../../api";
import type { TileBehavior } from "../../api";
import type { TileChange, TileJson, TileRole, TilesetDto } from "../../api.world";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { t as translate } from "../../i18n";
import { Button, Checkbox, InspectorRow, NumberField, Section, SelectField, TextField, checkNumber } from "../../ui/primitives";
import type { SelectOption } from "../../ui/primitives";
import { loadNames } from "../../world/scene/tileNames";
import { useSectionOpen } from "./foldState";
import styles from "./TileEditor.module.css";

/** The six roles in the order of the field „Rolle“. */
const ROLES: readonly TileRole[] = ["ground", "wall", "ramp", "water", "grass", "piece"];

/** The choices of the field „Rolle“, named in the language that is shown when called. */
const roleOptions = (): SelectOption<TileRole>[] => ROLES.map((role) => ({ value: role, label: translate(`workbench:tileEditor.role.${role}`) }));

/** Where a tile's defaults come from, as a sentence; the synthetic test tileset has none. */
const ORIGINS: ReadonlySet<TileJson["origin"]> = new Set(["vanilla", "unclear", "user"]);
const originText = (origin: TileJson["origin"]): string | undefined =>
  ORIGINS.has(origin) ? translate(`workbench:tileEditor.origin.${origin as "vanilla" | "unclear" | "user"}`) : undefined;

/** Bit 7 of the collision byte blocks the cell. */
const BLOCKED = 0x80;

/** The four hex digits of tile `index`, as the palette and the world document write them (`0041`). */
const digitsOf = (index: number) => index.toString(16).padStart(4, "0");
/** A byte in hex for Tüfteln („0x02“). */
const hex2 = (n: number) => `0x${n.toString(16).toUpperCase().padStart(2, "0")}`;

/**
 * The field „Wand“: a tile number as the palette writes it (hex digits), empty for none; an entry past the tileset is refused. The
 * edge set editor uses it for the wall of all tiles of a set.
 */
export function WallField({ wall, last, disabled, onCommit }: { wall: number | undefined; last: number; disabled: boolean; onCommit: (wall: number | null) => void }) {
  const { t } = useTranslation("workbench");
  const [error, setError] = useState<string | undefined>();
  useEffect(() => setError(undefined), [wall]);
  return (
    <TextField
      label={t("tileEditor.wall")}
      placeholder={t("tileEditor.none")}
      value={wall === undefined ? "" : digitsOf(wall)}
      error={error}
      disabled={disabled}
      onChange={() => setError(undefined)}
      onCommit={(text) => {
        if (text.trim() === "") {
          setError(undefined);
          if (wall !== undefined) onCommit(null);
          return;
        }
        const checked = checkNumber(text, "hex", 0, last);
        if (!checked.ok) return setError(t("tileEditor.wallRange", { last: digitsOf(last) }));
        setError(undefined);
        if (checked.value !== wall) onCommit(checked.value);
      }}
    />
  );
}

/**
 * The section „Kachel 0041“ of the chosen palette tile: its role, terrain (`kind`; Bauen by its behaviour name as the terrain overlay names
 * it, Tüfteln in hex), „Blockiert“ (bit 7 of the collision byte, the other bits stay), HeartGold/SoulSilver's step sound (0 to 127) and
 * the tile its walls take; under them where the defaults come from. Each confirmed field is one `tileset_edit` (one file step), after
 * which `onChanged` reads the tileset again. A derived tileset (`stored: false`) locks the fields and offers „Tileset im Projekt
 * anlegen“ (`tileset_save`) first. A refusal goes to `onError`.
 */
export function TileEditor({
  tileset,
  tile,
  family,
  depth,
  onChanged,
  onError,
  names = loadNames,
}: {
  tileset: TilesetDto;
  tile: number;
  family: "pt" | "hgss";
  depth: Depth;
  onChanged: () => void;
  onError: (e: unknown) => void;
  /** Where the behaviour names come from (tests, the gallery). */
  names?: () => Promise<ReadonlyMap<number, TileBehavior>>;
}): JSX.Element {
  const { t } = useTranslation("workbench");
  const [open, toggle] = useSectionOpen("matrixCell:tile", true);
  const tinker = depthAtLeast(depth, "tuefteln");
  const [behaviors, setBehaviors] = useState<ReadonlyMap<number, TileBehavior>>(new Map());
  const [working, setWorking] = useState(false);
  const report = useRef(onError);
  report.current = onError;
  useEffect(() => {
    let live = true;
    names().then(
      (map) => live && setBehaviors(map),
      (e: unknown) => live && report.current(e),
    );
    return () => {
      live = false;
    };
  }, [names]);
  const kinds = useMemo<SelectOption<number>[]>(() => [...behaviors.values()].sort((a, b) => a.value - b.value).map((b) => ({ value: b.value, label: b.name })), [behaviors]);

  const pkg = tileset.package;
  const tiles = tileset.tileset.tiles;
  const shown = tiles[tile];
  const locked = !tileset.stored || working;
  const digits = digitsOf(tile);

  async function run(op: () => Promise<unknown>) {
    if (working) return;
    setWorking(true);
    try {
      await op();
      onChanged();
    } catch (e) {
      report.current(e);
    } finally {
      setWorking(false);
    }
  }
  const edit = (change: Omit<TileChange, "tile">) => void run(() => tilesetEdit(pkg, [{ tile, ...change }]));

  const origin = shown ? originText(shown.origin) : undefined;
  const kindName = shown ? behaviors.get(shown.kind)?.name : undefined;
  return (
    <div data-tile-editor="">
      <Section title={t("tileEditor.title", { digits })} open={open} onToggle={toggle}>
        <div className={styles.body}>
          {!tileset.stored && (
            <div className={styles.save}>
              <Button kind="secondary" size="s" data-tileset-save="" aria-disabled={working || undefined} onClick={() => void run(() => tilesetSave(pkg))}>
                {t("edgeSetEditor.saveTileset")}
              </Button>
              <p className={styles.note}>{t("tileEditor.saveNote", { package: String(pkg).padStart(4, "0") })}</p>
            </div>
          )}
          {shown === undefined ? (
            <p className={styles.note}>{t("tileEditor.noTile", { digits })}</p>
          ) : (
            <>
              <InspectorRow label={t("tileEditor.roleLabel")}>
                <SelectField label={t("tileEditor.roleLabel")} value={shown.role} options={roleOptions()} disabled={locked} onChange={(role) => role !== shown.role && edit({ role })} />
              </InspectorRow>
              {tinker ? (
                <InspectorRow label={t("tileEditor.terrain")} hint={`${hex2(shown.kind)}${kindName ? ` ${kindName}` : ""}`}>
                  <NumberField label={t("tileEditor.terrain")} value={shown.kind} bits={8} base="hex" disabled={locked} onCommit={(kind) => edit({ kind })} />
                </InspectorRow>
              ) : (
                <InspectorRow label={t("tileEditor.terrain")}>
                  <SelectField label={t("tileEditor.terrain")} value={shown.kind} options={kinds} disabled={locked} onChange={(kind) => kind !== shown.kind && edit({ kind })} />
                </InspectorRow>
              )}
              <InspectorRow label={t("tileEditor.blocked")}>
                <Checkbox
                  label={t("tileEditor.blocked")}
                  checked={(shown.collision & BLOCKED) !== 0}
                  disabled={locked}
                  onChange={(on) => edit({ collision: on ? shown.collision | BLOCKED : shown.collision & ~BLOCKED })}
                />
              </InspectorRow>
              {family === "hgss" && (
                <InspectorRow label={t("tileEditor.stepSound")} hint={t("inspectorField.range", { max: 127 })}>
                  <NumberField label={t("tileEditor.stepSound")} value={shown.sound ?? 0} max={127} bits={8} disabled={locked} showBase={tinker} onCommit={(sound) => edit({ sound })} />
                </InspectorRow>
              )}
              <InspectorRow label={t("tileEditor.wall")} hint={t("tileEditor.wallHint")}>
                <WallField wall={shown.wall} last={Math.max(0, tiles.length - 1)} disabled={locked} onCommit={(wall) => edit({ wall })} />
              </InspectorRow>
              {origin && (
                <p className={styles.note} data-tile-origin={shown.origin}>
                  {origin}
                </p>
              )}
            </>
          )}
        </div>
      </Section>
    </div>
  );
}
