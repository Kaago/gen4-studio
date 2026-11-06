import { useEffect, useRef, useState } from "react";
import { getMapBudget } from "../../api";
import type { BudgetItemDto, MapBudgetDto } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Budget, Section } from "../../ui/primitives";
import { useSectionOpen } from "./foldState";
import styles from "./MapBudgetSection.module.css";

const number = (n: number) => n.toLocaleString("de-DE");
/** A limit in hex for Tüfteln („0xF000“). */
const hex = (n: number) => `0x${n.toString(16).toUpperCase()}`;

/** One budget row in bytes; Tüfteln adds the limit in hex. */
function ByteRow({ id, label, item, tinker, text, note }: { id: string; label: string; item: BudgetItemDto; tinker: boolean; text?: string; note?: string }) {
  return (
    <div data-budget={id}>
      <Budget label={label} used={item.used} max={item.max} state={item.state} unit="B" text={text} note={note} tech={tinker ? hex(item.max) : undefined} />
    </div>
  );
}

/**
 * The section „Budget“ of a map cell (Bauen and Tüfteln): the game's limits for the map — model, terrain (BDHC), buildings and the
 * texture VRAM of its area — each with its state, as `get_map_budget` reads them. Read again when the map or `revision` changes; an
 * answer for an older map or revision is dropped. The limits never block saving.
 */
export function MapBudgetSection({ map, revision, depth, onError }: { map: number; revision: number; depth: Depth; onError: (e: unknown) => void }) {
  const [open, toggle] = useSectionOpen("matrixCell:budget", true);
  // The budget and the map it belongs to: another map shows „Wird gelesen …“ until its own answer is there.
  const [shown, setShown] = useState<{ map: number; budget: MapBudgetDto | null } | null>(null);
  const report = useRef(onError);
  report.current = onError;
  useEffect(() => {
    let current = true;
    getMapBudget(map).then(
      (budget) => current && setShown({ map, budget }),
      (e: unknown) => {
        if (!current) return;
        setShown({ map, budget: null });
        report.current(e);
      },
    );
    return () => {
      current = false;
    };
  }, [map, revision]);
  const tinker = depthAtLeast(depth, "tuefteln");
  const budget = shown?.map === map ? shown.budget : undefined;
  const textures = budget?.textures ?? null;
  return (
    <div data-map-budget="">
      <Section title="Budget" open={open} onToggle={toggle}>
        <p className={styles.intro}>Grenzen des Spiels; Speichern geht immer.</p>
        {budget === undefined && <p className={styles.intro}>Wird gelesen …</p>}
        {budget && (
          <div className={styles.rows}>
            <ByteRow id="model" label="Modell" item={budget.model} tinker={tinker} />
            <ByteRow id="bdhc" label="Terrain (BDHC)" item={budget.bdhc} tinker={tinker} />
            <div data-budget="buildings">
              <Budget label="Gebäude" used={budget.buildings.used} max={budget.buildings.max} state={budget.buildings.state} />
            </div>
            {textures ? (
              <ByteRow
                id="vram"
                label="Texturen des Gebiets"
                item={textures.vram}
                tinker={tinker}
                text={`VRAM ${number(textures.vram.used)} von ${number(textures.vram.max)} B · ${number(textures.textures)} ${textures.textures === 1 ? "Textur" : "Texturen"}`}
                note={`Vanilla höchstens ${number(textures.vanilla_max)} B`}
              />
            ) : (
              budget.textures_missing && (
                <div data-budget="vram" className={styles.missing}>
                  <span className={styles.label}>Texturen des Gebiets</span>
                  <span>{budget.textures_missing}</span>
                </div>
              )
            )}
          </div>
        )}
      </Section>
    </div>
  );
}
