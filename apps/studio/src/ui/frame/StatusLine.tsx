import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
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
  /** The item of connected agents (`AgentStatus`), right before the build state. */
  agent?: ReactNode;
};

/** The catalog key of each density's name (`statusLine.<name>` in `ui`). */
const DENSITY_NAMES = { komfort: "densityComfort", kompakt: "densityCompact" } as const satisfies Record<Density, string>;

function BuildState({ builtAt, building }: StatusLineProps["build"]) {
  const { t } = useTranslation("ui");
  // Read again every half minute: the age is only as exact as "vor 4 Minuten".
  const now = useNow();
  if (building) {
    return (
      <span className={styles.build}>
        <Icon name="build" /> {t("statusLine.building")}
      </span>
    );
  }
  if (builtAt === null) return <span className={styles.build}>{t("statusLine.notBuilt")}</span>;
  return (
    <span className={styles.build}>
      <Icon name="check" /> {t("statusLine.built", { ago: ago(builtAt, now) })}
    </span>
  );
}

/**
 * The 28 px bar at the bottom: left the tool hints, in the middle the context, right the problems, the connected agents, the build, the
 * density and the bridge. `data-status-line` makes its footer the last stop of the F6 round (`useF6Ring`).
 */
export function StatusLine({ left, middle, right, bridge, density, onToggleDensity, build, agent }: StatusLineProps) {
  const { t } = useTranslation("ui");
  return (
    <div className={styles.line} data-status-line="">
      <div className={styles.left}>{left}</div>
      <div className={styles.middle}>{middle}</div>
      <div className={styles.right}>
        {right}
        {agent}
        <BuildState {...build} />
        <Tooltip content={t("statusLine.densityTip")}>
          <Button kind="quiet" size="s" onClick={onToggleDensity}>
            {t(`statusLine.${DENSITY_NAMES[density]}`)}
          </Button>
        </Tooltip>
        {bridge === "ok" && <span>{t("statusLine.bridgeOk")}</span>}
        {bridge === "missing" && <span className={styles.missing}>{t("statusLine.bridgeMissing")}</span>}
      </div>
    </div>
  );
}
