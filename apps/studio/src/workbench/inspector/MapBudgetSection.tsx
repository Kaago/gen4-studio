import { useEffect, useRef, useState } from "react";
import i18next from "i18next";
import { useTranslation } from "react-i18next";
import { getMapBudget } from "../../api";
import type { BudgetItemDto, MapBudgetDto } from "../../api";
import type { WorldBakeReport } from "../../api.world";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { t } from "../../i18n";
import { Budget, Section } from "../../ui/primitives";
import { useSectionOpen } from "./foldState";
import styles from "./MapBudgetSection.module.css";

/** A number with the thousands separator of the language that is shown („12.345“, "12,345"). */
const number = (n: number) => n.toLocaleString(i18next.language);
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
 * The sentence under „Modell“ when the last bake of a world map was refused: „Nicht gebacken: <the first error> Das Spiel bekommt das
 * letzte Modell.“; null for a bake that went through or none.
 */
function refusedText(bake: WorldBakeReport | null | undefined): string | null {
  if (!bake || bake.ok) return null;
  const first = bake.messages.find((m) => m.level === "error") ?? bake.messages[0];
  if (!first) return t("workbench:mapBudgetSection.notBaked");
  const message = first.message.trim();
  return t("workbench:mapBudgetSection.notBakedBecause", { message: /[.!?]$/.test(message) ? message : `${message}.` });
}

/**
 * The section „Budget“ of a map cell (Bauen and Tüfteln): the game's limits for the map — model, terrain (BDHC), buildings and the
 * texture VRAM of its area — each with its state, as `get_map_budget` reads them. Read again when the map or `revision` changes; an
 * answer for an older map or revision is dropped. The limits never block saving. The model of a world map is its bake: every stroke is a
 * revision, so the numbers follow the painting; a refused bake (`bake`, from `world_get`) says so under „Modell“.
 */
export function MapBudgetSection({
  map,
  revision,
  depth,
  onError,
  bake,
}: {
  map: number;
  revision: number;
  depth: Depth;
  onError: (e: unknown) => void;
  /** The last bake of a world map (`world_get.bake`); undefined or null says nothing. */
  bake?: WorldBakeReport | null;
}) {
  // Renders the section again when the language changes; the words are read at render time.
  useTranslation("workbench");
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
  const refused = refusedText(bake);
  return (
    <div data-map-budget="">
      <Section title={t("workbench:mapBudgetSection.title")} open={open} onToggle={toggle}>
        <p className={styles.intro}>{t("workbench:mapBudgetSection.intro")}</p>
        {budget === undefined && <p className={styles.intro}>{t("workbench:mapBudgetSection.reading")}</p>}
        {budget && (
          <div className={styles.rows}>
            <ByteRow id="model" label={t("workbench:mapBudgetSection.model")} item={budget.model} tinker={tinker} />
            {refused && (
              <p className={styles.refused} data-budget-refused="">
                {refused}
              </p>
            )}
            <ByteRow id="bdhc" label={t("workbench:mapBudgetSection.terrain")} item={budget.bdhc} tinker={tinker} />
            <div data-budget="buildings">
              <Budget label={t("workbench:mapBudgetSection.buildings")} used={budget.buildings.used} max={budget.buildings.max} state={budget.buildings.state} />
            </div>
            {textures ? (
              <ByteRow
                id="vram"
                label={t("workbench:mapBudgetSection.textures")}
                item={textures.vram}
                tinker={tinker}
                text={t("workbench:mapBudgetSection.vram", { used: number(textures.vram.used), max: number(textures.vram.max), textures: number(textures.textures), count: textures.textures })}
                note={t("workbench:mapBudgetSection.vanillaMax", { max: number(textures.vanilla_max) })}
              />
            ) : (
              budget.textures_missing && (
                <div data-budget="vram" className={styles.missing}>
                  <span className={styles.label}>{t("workbench:mapBudgetSection.textures")}</span>
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
