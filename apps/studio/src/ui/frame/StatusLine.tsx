import type { ReactNode } from "react";
import { Button, Icon, Tooltip } from "../primitives";
import type { Density } from "../tokens/theme";
import { ago, useNow } from "./timeAgo";
import styles from "./StatusLine.module.css";

export type StatusLineProps = {
  left: ReactNode;
  middle: ReactNode;
  right: ReactNode;
  /** "none": inside the app; "ok": the browser mode with its dev bridge; "missing": the bridge does not answer. */
  bridge: "none" | "ok" | "missing";
  density: Density;
  onToggleDensity: () => void;
  build: { builtAt: number | null; building: boolean };
};

const densityLabel: Record<Density, string> = { komfort: "Komfort", kompakt: "Kompakt" };

function BuildState({ builtAt, building }: StatusLineProps["build"]) {
  // Read again every half minute: the age is only as exact as "vor 4 Minuten".
  const now = useNow();
  if (building) {
    return (
      <span className={styles.build}>
        <Icon name="build" /> Baut …
      </span>
    );
  }
  if (builtAt === null) return <span className={styles.build}>Noch nicht gebaut</span>;
  return (
    <span className={styles.build}>
      <Icon name="check" /> Gebaut {ago(builtAt, now)}
    </span>
  );
}

/**
 * The 28 px bar at the bottom: left the tool hints, in the middle the context, right the problems, the build, the density and the bridge.
 * `data-status-line` makes its footer the last stop of the F6 round (`useF6Ring`).
 */
export function StatusLine({ left, middle, right, bridge, density, onToggleDensity, build }: StatusLineProps) {
  return (
    <div className={styles.line} data-status-line="">
      <div className={styles.left}>{left}</div>
      <div className={styles.middle}>{middle}</div>
      <div className={styles.right}>
        {right}
        <BuildState {...build} />
        <Tooltip content="Dichte wechseln (Komfort oder Kompakt)">
          <Button kind="quiet" size="s" onClick={onToggleDensity}>
            {densityLabel[density]}
          </Button>
        </Tooltip>
        {bridge === "ok" && <span>Browser-Modus (Brücke)</span>}
        {bridge === "missing" && <span className={styles.missing}>Brücke nicht erreichbar</span>}
      </div>
    </div>
  );
}
