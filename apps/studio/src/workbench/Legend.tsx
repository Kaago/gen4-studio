import { useId } from "react";
import { Icon, Popover } from "../ui/primitives";
import { legendEntries } from "../world/legend";
import type { TerrainEntry } from "../world/legend";
import type { Pattern } from "../world/scene/patterns";
import styles from "./Legend.module.css";

const ENTRIES = legendEntries();

function Swatch({ color }: { color: string }) {
  return <span className={styles.swatch} style={{ background: color }} /* dyn */ aria-hidden="true" />;
}

/** Wavy line `k` of the pattern „Wellen“ in a 12 × 12 box (y down = south), sampled like the shader's sine. */
function wave(k: number): string {
  const points: string[] = [];
  for (let x = 0; x <= 12; x++) points.push(`${x},${(12 * (1 - (k + 0.5 - 0.15 * Math.sin((4 * Math.PI * x) / 12)) / 3)).toFixed(2)}`);
  return points.join(" ");
}

const STRIPES = [-9, -6, -3, 0, 3, 6, 9];

/** The marks of a pattern in a 12 × 12 box, north up, as the scene draws them on a cell (the arrow points north). */
function PatternMarks({ pattern, color }: { pattern: Pattern; color: string }) {
  const hatch = STRIPES.map((d) => <line key={`h${d}`} x1={d} y1={0} x2={d + 12} y2={12} stroke={color} strokeWidth={1} />);
  switch (pattern) {
    case "hatch":
      return <>{hatch}</>;
    case "crosshatch":
      return (
        <>
          {hatch}
          {STRIPES.map((d) => (
            <line key={`c${d}`} x1={d + 12} y1={0} x2={d} y2={12} stroke={color} strokeWidth={1} />
          ))}
        </>
      );
    case "waves":
      return (
        <>
          {[0, 1, 2].map((k) => (
            <polyline key={k} points={wave(k)} fill="none" stroke={color} strokeWidth={1} />
          ))}
        </>
      );
    case "dots":
      return (
        <>
          {[2, 6, 10].flatMap((y) => [2, 6, 10].map((x) => <circle key={`${x},${y}`} cx={x} cy={y} r={1.44} fill={color} />))}
        </>
      );
    case "arrow":
      return <path d="M6 1.68 L8.7 5.28 L6.72 5.28 L6.72 10.08 L5.28 10.08 L5.28 5.28 L3.3 5.28 Z" fill={color} />;
    case "none":
      return null;
  }
}

/** The swatch of a terrain class: its colour with the pattern of the layer „Gelände“ over it (`data-pattern`). */
export function PatternSwatch({ entry }: { entry: Pick<TerrainEntry, "color" | "pattern" | "patternColor"> }) {
  return (
    <span className={styles.swatch} style={{ background: entry.color }} /* dyn */ data-pattern={entry.pattern} aria-hidden="true">
      {entry.patternColor && (
        <svg className={styles.pattern} viewBox="0 0 12 12" preserveAspectRatio="none">
          <PatternMarks pattern={entry.pattern} color={entry.patternColor} />
        </svg>
      )}
    </span>
  );
}

/** The legend of the scene at the bottom left of the 3D view: every colour (terrain with its pattern) with its name (never the colour alone). */
export function Legend() {
  const id = useId();
  return (
    <div className={styles.legend}>
      <Popover
        trigger={
          <>
            <Icon name="legend" />
            Legende
          </>
        }
      >
        <section className={styles.group} aria-labelledby={`${id}-terrain`}>
          <h3 id={`${id}-terrain`} className={styles.heading}>
            Gelände
          </h3>
          <ul className={styles.list}>
            {ENTRIES.terrain.map((t) => (
              <li key={t.key} className={styles.entry}>
                <PatternSwatch entry={t} />
                {t.label}
              </li>
            ))}
          </ul>
        </section>
        <section className={styles.group} aria-labelledby={`${id}-events`}>
          <h3 id={`${id}-events`} className={styles.heading}>
            Ereignisse
          </h3>
          <ul className={styles.list}>
            {ENTRIES.events.map((e) => (
              <li key={e.kind} className={styles.entry}>
                <Swatch color={e.color} />
                <Icon name={e.icon} />
                {e.label}
              </li>
            ))}
          </ul>
        </section>
      </Popover>
    </div>
  );
}
