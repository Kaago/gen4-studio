import type { MelonDsConfigDto, MelonDsDto } from "../../api.live";
import styles from "./LiveRow.module.css";

export type LiveRowProps = {
  /** The melonDS of the last „Hier spielen“ (`live_status.melonds`); null or undefined while none is known. */
  melonds: MelonDsDto | null | undefined;
};

/** How the next „Hier spielen“ fixes a setting that keeps melonDS from opening a usable GDB stub. */
const WHEN = "beim nächsten „Hier spielen“";
const UNLESS_RUNNING = "wenn melonDS nicht läuft";

/** What is missing for a usable stub, each with what the next „Hier spielen“ does about it; empty when the stub is ready. */
function missing(config: MelonDsConfigDto): string[] {
  const out: string[] = [];
  if (!config.enabled) out.push(`GDB-Stub aus: wird ${WHEN} eingeschaltet, ${UNLESS_RUNNING}`);
  if (config.break_on_startup) out.push(`Halt beim Start an: wird ${WHEN} ausgeschaltet, ${UNLESS_RUNNING}`);
  if (config.jit) out.push(`JIT an: wird ${WHEN} ausgeschaltet, ${UNLESS_RUNNING}`);
  return out;
}

/** The one line of the state: version, stub, port, JIT („melonDS 1.1 · GDB-Stub an · Port 3333 · JIT aus“). */
function summary(m: MelonDsDto): string {
  const c = m.config;
  if (c === null) return `melonDS ${m.version} · Einstellungen nicht lesbar`;
  return [`melonDS ${m.version}`, c.enabled ? "GDB-Stub an" : "GDB-Stub aus", `Port ${c.port}`, c.jit ? "JIT an" : "JIT aus"].join(" · ");
}

/**
 * „Spielmodus live“ (Technik › Emulator): whether the melonDS of the last „Hier spielen“ opens the GDB stub the studio connects to — its
 * version, the stub, its port and the JIT, or what is missing and that the next „Hier spielen“ sets it — and where its settings live.
 */
export function LiveRow({ melonds }: LiveRowProps) {
  return (
    <div className={styles.row} data-live-row="">
      <span className={styles.label}>Spielmodus live</span>
      {melonds === null || melonds === undefined ? (
        <span className={styles.quiet}>Kein melonDS gewählt</span>
      ) : (
        <>
          <span className={styles.state}>{summary(melonds)}</span>
          {melonds.config !== null &&
            missing(melonds.config).map((line) => (
              <span key={line} className={styles.quiet}>
                {line}
              </span>
            ))}
          <span className={styles.path} title={melonds.config_path}>
            {melonds.config_path}
          </span>
        </>
      )}
    </div>
  );
}
